use std::{
    collections::{BTreeMap, BTreeSet},
    ops::{Deref, DerefMut, Index, IndexMut},
    time::Duration,
};

use dango_client::SingleSigner;
use dango_hyperlane_types::isms;
use dango_types::{
    auth::Nonce,
    config::AppConfig,
    warp::{self, Route},
};
use grug::{
    Addr, Coin, Coins, Defined, Denom, GasOption, HexByteArray, Message, SigningClient, TxOutcome,
};
use hyperlane_dango::{DangoProviderInterface, TryDangoConvertor};
use tokio::time::sleep;

pub struct ChainHelper {
    pub cfg: AppConfig,
    pub client: SigningClient,
    pub accounts: Accounts,
}

impl ChainHelper {
    pub async fn new(
        client: SigningClient,
        accounts: BTreeMap<String, SingleSigner<Defined<Nonce>>>,
    ) -> anyhow::Result<Self> {
        let cfg = client.query_app_config().await?;

        Ok(Self {
            cfg,
            client,
            accounts: Accounts(accounts),
        })
    }

    pub async fn set_route(
        &mut self,
        denom: Denom,
        destination_domain: u32,
        route: Route,
    ) -> anyhow::Result<TxOutcome> {
        self.boradcast_and_find(
            "owner",
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
        )
        .await
    }

    pub async fn send_remote(
        &mut self,
        sender: &str,
        coin: Coin,
        destination_domain: u32,
        recipient: Addr,
    ) -> anyhow::Result<TxOutcome> {
        self.boradcast_and_find(
            sender,
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
        )
        .await
    }

    pub async fn set_hyperlane_validators(
        &mut self,
        remote_domain: u32,
        threshold: u32,
        validators: BTreeSet<HexByteArray<20>>,
    ) -> anyhow::Result<TxOutcome> {
        self.boradcast_and_find(
            "owner",
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
        )
        .await
    }

    async fn boradcast_and_find(
        &mut self,
        signer: &str,
        msg: Message,
        gas_opt: GasOption,
    ) -> anyhow::Result<TxOutcome> {
        let hash = self
            .client
            .send_message(&mut self.accounts[signer], msg, gas_opt)
            .await?
            .hash
            .try_convert()?;

        loop {
            let outcome = self.client.search_tx(hash).await;

            if let Ok(outcome) = outcome {
                return Ok(outcome.outcome);
            }

            sleep(Duration::from_secs(1)).await;
        }
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
