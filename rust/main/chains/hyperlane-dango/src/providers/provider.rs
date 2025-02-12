use {
    super::{graphql::GraphQlProvider, DangoProviderInterface},
    crate::{
        BlockOutcome, BlockResultOutcome, ConnectionConf, DangoResult, DangoSigner, HashConvertor,
        IntoDangoError, ProviderConf, SearchTxOutcome, SimulateOutcome, TryHashConvertor,
    },
    anyhow::anyhow,
    async_trait::async_trait,
    dango_types::{account::spot, auth::Metadata},
    grug::{
        Addr, Coin, ContractInfo, Defined, Denom, GasOption, Hash256, Inner, JsonDeExt, Message,
        QueryRequest, Signer, SigningClient, TxOutcome, Uint128,
    },
    hyperlane_core::{
        BlockInfo, ChainInfo, ChainResult, HyperlaneChain, HyperlaneDomain, HyperlaneProvider,
        TxnInfo, H256, H512, U256,
    },
    serde::{de::DeserializeOwned, Serialize},
    std::{
        fmt::Debug,
        ops::{Deref, DerefMut, RangeInclusive},
        str::FromStr,
    },
};

const MAX_SEARCH_TX_ATTEMPTS: u64 = 10;
const SEARC_SLEEP_DURATION: u64 = 1;

#[derive(Debug, Clone)]
pub struct DangoProvider {
    pub domain: HyperlaneDomain,
    pub connection_conf: ConnectionConf,
    pub signer: Option<DangoSigner>,
    pub provider: ProviderWrapper,
}

impl HyperlaneChain for DangoProvider {
    fn domain(&self) -> &HyperlaneDomain {
        &self.domain
    }

    fn provider(&self) -> Box<dyn HyperlaneProvider> {
        Box::new(self.clone())
    }
}

#[async_trait]
impl HyperlaneProvider for DangoProvider {
    /// Get block info for a given block height
    async fn get_block_by_height(&self, height: u64) -> ChainResult<BlockInfo> {
        let block = self.provider.get_block(Some(height)).await?;

        Ok(BlockInfo {
            hash: block.hash.convert(),
            timestamp: block.timestamp,
            number: block.height,
        })
    }

    /// Get txn info for a given txn hash
    async fn get_txn_by_hash(&self, hash: &H512) -> ChainResult<TxnInfo> {
        let tx = self.provider.search_tx(hash.try_convert()?).await?;

        let data: Metadata = tx.tx.data.deserialize_json().into_dango_error()?;

        Ok(TxnInfo {
            hash: *hash,
            gas_limit: tx.outcome.gas_limit.into(),
            max_priority_fee_per_gas: None,
            max_fee_per_gas: None,
            // TODO: is this needed?
            // This function seems to used only by scraper
            gas_price: None,
            nonce: data.nonce.into(),
            sender: tx.tx.sender.convert(),
            // TODO: is this needed (should be the contract)?
            recipient: None,
            receipt: None,
            raw_input_data: None,
        })
    }

    /// Returns whether a contract exists at the provided address
    async fn is_contract(&self, address: &H256) -> ChainResult<bool> {
        match self.provider.contract_info(address.try_convert()?).await {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }

    /// Fetch the balance of the wallet address associated with the chain provider.
    async fn get_balance(&self, address: String) -> ChainResult<U256> {
        let address = Addr::from_str(&address).into_dango_error()?;

        let balance = self
            .provider
            .balance(address, self.connection_conf.get_canonical_asset().clone())
            .await?;

        Ok(balance.into_inner().into())
    }

    /// Fetch metrics related to this chain
    async fn get_chain_metrics(&self) -> ChainResult<Option<ChainInfo>> {
        let block = self.provider.get_block(None).await?;
        return Ok(Some(ChainInfo {
            latest_block: BlockInfo {
                hash: block.hash.convert(),
                timestamp: block.timestamp,
                number: block.height,
            },
            min_gas_price: None,
        }));
    }
}

impl DangoProvider {
    pub fn from_config(
        config: &ConnectionConf,
        domain: HyperlaneDomain,
        signer: Option<DangoSigner>,
    ) -> DangoResult<Self> {
        match &config.provider_conf {
            ProviderConf::Rpc(rpc_config) => Ok(DangoProvider {
                domain,
                provider: ProviderWrapper::Rpc(SigningClient::connect(
                    rpc_config.chain_id.clone(),
                    rpc_config.url.as_str(),
                )?),
                connection_conf: config.clone(),
                signer,
            }),
            // TODO: DANGO
            ProviderConf::GraphQl(_) => unimplemented!(),
        }
    }

    pub fn gas_price(&self) -> &Coin {
        &self.connection_conf.gas_price
    }

    pub async fn get_block(&self, height: Option<u64>) -> DangoResult<BlockOutcome> {
        self.provider.get_block(height).await
    }

    /// Get transaction info for a given transaction hash.
    pub async fn search_tx(&self, hash: Hash256) -> DangoResult<SearchTxOutcome> {
        self.provider.search_tx(hash).await
    }

    pub async fn search_tx_loop(&self, hash: Hash256) -> DangoResult<SearchTxOutcome> {
        for _ in 0..MAX_SEARCH_TX_ATTEMPTS {
            let search_result = self.search_tx(hash).await?;

            if search_result.outcome.result.is_ok() {
                return Ok(search_result);
            }

            tokio::time::sleep(tokio::time::Duration::from_secs(SEARC_SLEEP_DURATION)).await;
        }

        Err(crate::DangoError::TxNotFound { hash })
    }

    /// Get the balance of an address for a given denom.
    pub async fn balance(&self, addr: Addr, denom: Denom) -> DangoResult<Uint128> {
        self.provider.balance(addr, denom).await
    }

    /// Get the contract info for a given contract address.
    pub async fn contract_info(&self, addr: Addr) -> DangoResult<ContractInfo> {
        self.provider.contract_info(addr).await
    }

    /// Query a wasm smart contract.
    pub async fn query_wasm_smart<R>(
        &self,
        contract: Addr,
        req: R,
        height: Option<u64>,
    ) -> DangoResult<R::Response>
    where
        R: QueryRequest + Send + Sync + 'static,
        R::Message: Serialize + Send + Sync + 'static,
        R::Response: DeserializeOwned,
    {
        self.provider.query_wasm_smart(contract, req, height).await
    }

    /// Sign and broadcast a message.
    pub async fn send_message(&self, msg: Message, gas_limit: Option<u64>) -> DangoResult<Hash256> {
        let signer = self.signer()?;

        let nonce = self
            .provider
            .query_wasm_smart(
                signer.read().await.address,
                spot::QuerySeenNoncesRequest {},
                None,
            )
            .await?
            .last()
            .map(|newest_nonce| newest_nonce + 1)
            .unwrap_or(0);

        signer.write().await.nonce = Defined::new(nonce);

        let gas = if let Some(gas_limit) = gas_limit {
            GasOption::Predefined { gas_limit }
        } else {
            GasOption::Simulate {
                scale: self.connection_conf.gas_scale,
                flat_increase: self.connection_conf.flat_gas_increase,
            }
        };

        let response = self
            .provider
            .send_message(signer.write().await.deref_mut(), msg, gas)
            .await?;

        Ok(response)
    }

    /// Sign and broadcast a message.
    pub async fn send_message_and_find(
        &self,
        msg: Message,
        gas_limit: Option<u64>,
    ) -> DangoResult<hyperlane_core::TxOutcome> {
        let hash = self.send_message(msg, gas_limit).await?;
        let outcome = self.search_tx_loop(hash).await?;
        return Ok(outcome.into_hyperlane_outcome(hash, self.gas_price()));
    }

    pub async fn simulate_message(&self, msg: Message) -> DangoResult<SimulateOutcome> {
        let tx_outcome = self
            .provider
            .simulate_message(self.signer()?.read().await.deref(), msg)
            .await?;

        Ok(SimulateOutcome {
            gas_adjusted: (tx_outcome.gas_used as f64 * self.connection_conf.gas_scale) as u64
                + self.connection_conf.flat_gas_increase,
            outcome: tx_outcome,
        })
    }

    pub async fn estimate_costs(
        &self,
        msg: Message,
    ) -> DangoResult<hyperlane_core::TxCostEstimate> {
        let outcome = self.simulate_message(msg).await?;

        Ok(hyperlane_core::TxCostEstimate {
            gas_limit: outcome.gas_adjusted.into(),
            gas_price: self.gas_price().amount.inner().into(),
            l2_gas_limit: None,
        })
    }

    // pub async fn fetch_logs(&self, range: RangeInclusive<u32>) {
    //     for i in range {
    //         let block = sel
    //         let logs = self.provider.get_block_result(i).await;
    //     }
    // }

    async fn get_block_full(self, height: u64) -> DangoResult<Vec<SearchTxOutcome>>{
        let block = self.provider.get_block(Some(height)).await?;
        let block_result = self.provider.get_block_result(Some(height)).await?;
        let mut txs = Vec::new();

        block.txs.into_iter().zip(block_result.txs).map(|(tx,tx_outcome)| {
            txs.push(SearchTxOutcome {
                tx,
                outcome: tx_outcome,
            });
        });

        Ok(txs)
    }

    fn signer(&self) -> DangoResult<DangoSigner> {
        Ok(self
            .signer
            .clone()
            .ok_or(anyhow!("can't use send_message if signer is not specified"))?)
    }
}

#[derive(Debug, Clone)]
pub enum ProviderWrapper {
    Rpc(SigningClient),
    GraphQl(GraphQlProvider),
}

impl ProviderWrapper {
    /// Get block info for a given block height. If block height is None, return the latest block.
    pub async fn get_block(&self, height: Option<u64>) -> DangoResult<BlockOutcome> {
        match self {
            ProviderWrapper::Rpc(provider) => provider.get_block(height).await,
            ProviderWrapper::GraphQl(provider) => provider.get_block(height).await,
        }
    }

    pub async fn get_block_result(&self, height: Option<u64>) -> DangoResult<BlockResultOutcome> {
        match self {
            ProviderWrapper::Rpc(provider) => provider.get_block_result(height).await,
            ProviderWrapper::GraphQl(provider) => provider.get_block_result(height).await,
        }
    }

    /// Get transaction info for a given transaction hash.
    pub async fn search_tx(&self, hash: Hash256) -> DangoResult<SearchTxOutcome> {
        match self {
            ProviderWrapper::Rpc(provider) => provider.search_tx(hash).await,
            ProviderWrapper::GraphQl(provider) => provider.search_tx(hash).await,
        }
    }

    /// Get the balance of an address for a given denom.
    pub async fn balance(&self, addr: Addr, denom: Denom) -> DangoResult<Uint128> {
        match self {
            ProviderWrapper::Rpc(provider) => provider.balance(addr, denom).await,
            ProviderWrapper::GraphQl(provider) => provider.balance(addr, denom).await,
        }
    }

    /// Get the contract info for a given contract address.
    pub async fn contract_info(&self, addr: Addr) -> DangoResult<ContractInfo> {
        match self {
            ProviderWrapper::Rpc(provider) => provider.contract_info(addr).await,
            ProviderWrapper::GraphQl(provider) => provider.contract_info(addr).await,
        }
    }

    /// Query a wasm smart contract.
    pub async fn query_wasm_smart<R>(
        &self,
        contract: Addr,
        req: R,
        height: Option<u64>,
    ) -> DangoResult<R::Response>
    where
        R: QueryRequest + Send + Sync + 'static,
        R::Message: Serialize + Send + Sync + 'static,
        R::Response: DeserializeOwned,
    {
        match self {
            ProviderWrapper::Rpc(provider) => {
                provider.query_wasm_smart(contract, req, height).await
            }
            ProviderWrapper::GraphQl(provider) => {
                provider.query_wasm_smart(contract, req, height).await
            }
        }
    }

    /// Sign and broadcast a message.
    pub async fn send_message<S>(
        &self,
        signer: &mut S,
        msg: Message,
        gas: GasOption,
    ) -> DangoResult<Hash256>
    where
        S: Signer + Send + Sync,
    {
        match self {
            ProviderWrapper::Rpc(provider) => {
                DangoProviderInterface::send_message(provider, signer, msg, gas).await
            }
            ProviderWrapper::GraphQl(provider) => provider.send_message(signer, msg, gas).await,
        }
    }

    pub async fn simulate_message<S>(&self, signer: &S, msg: Message) -> DangoResult<TxOutcome>
    where
        S: Signer + Send + Sync,
    {
        match self {
            ProviderWrapper::Rpc(provider) => provider.simulate_message(signer, msg).await,
            ProviderWrapper::GraphQl(provider) => provider.simulate_message(signer, msg).await,
        }
    }
}
