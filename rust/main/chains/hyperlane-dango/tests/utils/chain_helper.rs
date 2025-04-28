use {
    async_trait::async_trait,
    dango_client::SingleSigner,
    dango_hyperlane_types::isms,
    dango_types::{
        auth::Nonce,
        config::AppConfig,
        warp::{self, Route},
    },
    grug::{
        Addr, BroadcastClientExt, Coin, Coins, Defined, Denom, GasOption, HexByteArray, Message,
        QueryClientExt, SearchTxClient, SearchTxOutcome, Signer, TendermintRpcClient, TxOutcome,
    },
    std::{
        collections::{BTreeMap, BTreeSet},
        ops::{Deref, DerefMut, Index, IndexMut},
        time::Duration,
    },
    tokio::time::sleep,
};

pub struct ChainHelper {
    pub cfg: AppConfig,
    pub client: TendermintRpcClient,
    pub accounts: Accounts,
    pub chain_id: String,
    pub hyperlane_domain: u32,
}

impl ChainHelper {
    pub async fn new(
        client: TendermintRpcClient,
        accounts: BTreeMap<String, SingleSigner<Defined<Nonce>>>,
        chain_id: String,
        hyperlane_domain: u32,
    ) -> anyhow::Result<Self> {
        let cfg = client.query_app_config(None).await?;

        Ok(Self {
            cfg,
            client,
            accounts: Accounts(accounts),
            chain_id,
            hyperlane_domain,
        })
    }

    pub async fn set_route(
        &mut self,
        denom: Denom,
        destination_domain: u32,
        route: Route,
    ) -> anyhow::Result<TxOutcome> {
        Ok(self
            .client
            .broadcast_and_find(
                &mut self.accounts["owner"],
                Message::execute(
                    self.cfg.addresses.warp,
                    &warp::ExecuteMsg::SetRoute {
                        denom: denom.clone(),
                        destination_domain,
                        route: route.clone(),
                    },
                    Coins::default(),
                )?,
                GasOption::Predefined {
                    gas_limit: 10_000_000,
                },
                &self.chain_id,
            )
            .await?
            .outcome)
    }

    pub async fn send_remote(
        &mut self,
        sender: &str,
        coin: Coin,
        destination_domain: u32,
        recipient: Addr,
    ) -> anyhow::Result<SearchTxOutcome> {
        self.client
            .broadcast_and_find(
                &mut self.accounts[sender],
                Message::execute(
                    self.cfg.addresses.warp,
                    &warp::ExecuteMsg::TransferRemote {
                        destination_domain,
                        recipient: recipient.into(),
                        metadata: None,
                    },
                    coin.clone(),
                )?,
                GasOption::Predefined {
                    gas_limit: 10_000_000,
                },
                &self.chain_id,
            )
            .await
    }

    pub async fn set_hyperlane_validators(
        &mut self,
        remote_domain: u32,
        threshold: u32,
        validators: BTreeSet<HexByteArray<20>>,
    ) -> anyhow::Result<TxOutcome> {
        Ok(self
            .client
            .broadcast_and_find(
                &mut self.accounts["owner"],
                Message::execute(
                    self.cfg.addresses.hyperlane.ism,
                    &isms::multisig::ExecuteMsg::SetValidators {
                        domain: remote_domain,
                        threshold,
                        validators,
                    },
                    Coins::default(),
                )?,
                GasOption::Predefined {
                    gas_limit: 10_000_000,
                },
                &self.chain_id,
            )
            .await?
            .outcome)
    }
}
pub struct Accounts(BTreeMap<String, SingleSigner<Defined<Nonce>>>);

impl Deref for Accounts {
    type Target = BTreeMap<String, SingleSigner<Defined<Nonce>>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Accounts {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<S> Index<S> for Accounts
where
    S: AsRef<str>,
{
    type Output = SingleSigner<Defined<Nonce>>;

    fn index(&self, index: S) -> &Self::Output {
        self.get(index.as_ref()).expect("account not found")
    }
}

impl<S> IndexMut<S> for Accounts
where
    S: AsRef<str>,
{
    fn index_mut(&mut self, index: S) -> &mut Self::Output {
        self.get_mut(index.as_ref()).expect("account not found")
    }
}

#[async_trait]
pub trait ClientExt {
    async fn broadcast_and_find<S>(
        &self,
        signer: &mut S,
        message: Message,
        gas_opt: GasOption,
        chain_id: &str,
    ) -> anyhow::Result<SearchTxOutcome>
    where
        S: Signer + Send + Sync + 'static;
}

#[async_trait]
impl ClientExt for TendermintRpcClient {
    async fn broadcast_and_find<S>(
        &self,
        signer: &mut S,
        message: Message,
        gas_opt: GasOption,
        chain_id: &str,
    ) -> anyhow::Result<SearchTxOutcome>
    where
        S: Signer + Send + Sync + 'static,
    {
        let broadcast_outcome = self
            .send_message(signer, message, gas_opt, chain_id)
            .await?
            .into_result()
            .map_err(|e| anyhow::anyhow!(e.check_tx.error))?;

        let hash = broadcast_outcome.tx_hash;

        let mut counter = 0;

        while counter < 10 {
            let outcome = self.search_tx(hash).await;

            if let Ok(outcome) = outcome {
                return Ok(outcome);
            }

            counter += 1;
            sleep(Duration::from_secs(1)).await;
        }

        Err(anyhow::anyhow!("error while broadcasting tx: {}", hash))
    }
}
