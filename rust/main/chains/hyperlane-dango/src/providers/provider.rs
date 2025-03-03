use {
    super::{graphql::GraphQlProvider, DangoProviderInterface},
    crate::{
        BlockLogs, BlockOutcome, BlockResultOutcome, ConnectionConf, DangoConvertor, DangoResult,
        DangoSigner, ExecutionBlock, IntoDangoError, ProviderConf, SearchTxOutcome,
        SimulateOutcome, TryDangoConvertor,
    },
    anyhow::anyhow,
    async_trait::async_trait,
    dango_types::{account::spot, auth::Metadata},
    futures_util::future::try_join_all,
    grug::{
        Addr, ContractInfo, Defined, Denom, GasOption, Hash256, Inner, JsonDeExt, Message,
        QueryRequest, SigningClient, Uint128,
    },
    hyperlane_core::{
        BlockInfo, ChainInfo, ChainResult, HyperlaneChain, HyperlaneDomain, HyperlaneProvider,
        ReorgPeriod, TxnInfo, H256, H512, U256,
    },
    serde::{de::DeserializeOwned, Serialize},
    std::{
        fmt::Debug,
        ops::{Deref, DerefMut, RangeInclusive},
        str::FromStr,
    },
};

macro_rules! use_provider {
    ($self:ident, $method:ident ($($args:expr),*)) => {
        match &$self.provider {
            ProviderWrapper::Rpc(provider) => provider.$method($($args),*).await,
            ProviderWrapper::GraphQl(provider) => provider.$method($($args),*).await,
        }
    };
}

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
        let block = self.get_block(Some(height)).await?;

        Ok(BlockInfo {
            hash: block.hash.convert(),
            timestamp: block.timestamp,
            number: block.height,
        })
    }

    /// Get txn info for a given txn hash
    async fn get_txn_by_hash(&self, hash: &H512) -> ChainResult<TxnInfo> {
        let tx = self.search_tx(hash.try_convert()?).await?;

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
        match self.contract_info(address.try_convert()?).await {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }

    /// Fetch the balance of the wallet address associated with the chain provider.
    async fn get_balance(&self, address: String) -> ChainResult<U256> {
        let address = Addr::from_str(&address).into_dango_error()?;

        let balance = self
            .balance(address, self.connection_conf.gas_price.denom.clone())
            .await?;

        Ok(balance.into_inner().into())
    }

    /// Fetch metrics related to this chain
    async fn get_chain_metrics(&self) -> ChainResult<Option<ChainInfo>> {
        let block = self.get_block(None).await?;
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
            ProviderConf::Rpc(_) => Ok(DangoProvider {
                domain,
                provider: ProviderWrapper::Rpc(SigningClient::connect(
                    config.chain_id.clone(),
                    config
                        .rpcs
                        .first()
                        .ok_or(anyhow!("rpcs is empty"))?
                        .as_str(),
                )?),
                connection_conf: config.clone(),
                signer,
            }),
            // TODO: DANGO
            ProviderConf::GraphQl(_) => unimplemented!(),
        }
    }

    // Query

    /// Get block info for a given block height. If block height is None, return the latest block.
    pub async fn get_block(&self, height: Option<u64>) -> DangoResult<BlockOutcome> {
        use_provider!(self, get_block(height))
    }

    /// Get block result for a given block height. If block height is None, return the latest block.
    pub async fn get_block_result(&self, height: Option<u64>) -> DangoResult<BlockResultOutcome> {
        use_provider!(self, get_block_result(height))
    }

    /// Get the balance of an address for a given denom.
    pub async fn balance(&self, addr: Addr, denom: Denom) -> DangoResult<Uint128> {
        use_provider!(self, balance(addr, denom))
    }

    /// Get transaction info for a given transaction hash.
    pub async fn search_tx(&self, hash: Hash256) -> DangoResult<SearchTxOutcome> {
        use_provider!(self, search_tx(hash))
    }

    /// Get transaction info for a given transaction hash and retry if it is not found.
    pub async fn search_tx_loop(&self, hash: Hash256) -> DangoResult<SearchTxOutcome> {
        for _ in 0..self.connection_conf.search_retry_attempts {
            if let Ok(result) = self.search_tx(hash).await {
                return Ok(result);
            }

            tokio::time::sleep(tokio::time::Duration::from_secs(
                self.connection_conf.search_sleep_duration,
            ))
            .await;
        }

        Err(crate::DangoError::TxNotFound { hash })
    }

    /// Get the contract info for a given contract address.
    pub async fn contract_info(&self, addr: Addr) -> DangoResult<ContractInfo> {
        use_provider!(self, contract_info(addr))
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
        use_provider!(self, query_wasm_smart(contract, req, height))
    }

    /// Query the chain config.
    pub async fn query_app_config<T>(&self) -> DangoResult<T>
    where
        T: DeserializeOwned,
    {
        use_provider!(self, query_app_config())
    }

    /// Simulate a message.
    pub async fn simulate_message(&self, msg: Message) -> DangoResult<SimulateOutcome> {
        let tx_outcome = use_provider!(
            self,
            simulate_message(self.signer()?.read().await.deref(), msg)
        )?;

        Ok(SimulateOutcome {
            gas_adjusted: (tx_outcome.gas_used as f64 * self.connection_conf.gas_scale) as u64
                + self.connection_conf.flat_gas_increase,
            outcome: tx_outcome,
        })
    }

    /// Estimate the costs of a message.
    pub async fn estimate_costs(
        &self,
        msg: Message,
    ) -> DangoResult<hyperlane_core::TxCostEstimate> {
        let outcome = self.simulate_message(msg).await?;

        Ok(hyperlane_core::TxCostEstimate {
            gas_limit: outcome.gas_adjusted.into(),
            gas_price: self.connection_conf.gas_price.amount.inner().into(),
            l2_gas_limit: None,
        })
    }

    // Execute

    /// Sign and broadcast a message.
    pub async fn send_message(&self, msg: Message, gas_limit: Option<u64>) -> DangoResult<Hash256> {
        let signer = self.signer()?;

        let nonce = self
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

        use_provider!(
            self,
            broadcast_message(signer.write().await.deref_mut(), msg, gas)
        )
    }

    /// Sign and broadcast a message.
    pub async fn send_message_and_find(
        &self,
        msg: Message,
        gas_limit: Option<u64>,
    ) -> DangoResult<hyperlane_core::TxOutcome> {
        let hash = self.send_message(msg, gas_limit).await?;
        let outcome = self.search_tx_loop(hash).await?;
        return Ok(outcome.into_hyperlane_outcome(hash, &self.connection_conf.gas_price));
    }

    // Utility

    /// Get the block height for a given reorg period.
    pub async fn get_block_height_by_reorg_period(
        &self,
        reorg_period: ReorgPeriod,
    ) -> DangoResult<Option<u64>> {
        let block_height = match reorg_period {
            ReorgPeriod::Blocks(blocks) => {
                let last_block = self.get_block(None).await?;
                let block_height = last_block.height - blocks.get() as u64;
                Some(block_height)
            }
            ReorgPeriod::None => None,
            ReorgPeriod::Tag(_) => {
                return Err(anyhow::anyhow!(
                    "Tag reorg period is not supported in Dango MerkleTreeHook"
                )
                .into())
            }
        };

        Ok(block_height)
    }

    /// Get the block height for a given execution block.
    pub async fn get_block_height_by_execution_block(
        &self,
        execution_block: ExecutionBlock,
    ) -> DangoResult<Option<u64>> {
        match execution_block {
            ExecutionBlock::ReorgPeriod(reorg_period) => {
                self.get_block_height_by_reorg_period(reorg_period).await
            }
            ExecutionBlock::Defined(height) => Ok(Some(height)),
        }
    }

    pub async fn fetch_logs(&self, range: RangeInclusive<u32>) -> DangoResult<Vec<BlockLogs>> {
        let tasks = range
            .into_iter()
            .map(|i| async move { self.get_block_logs(i as u64).await })
            .collect::<Vec<_>>();

        try_join_all(tasks).await
    }

    async fn get_block_logs(&self, height: u64) -> DangoResult<BlockLogs> {
        let block = self.get_block(Some(height)).await?;
        let block_result = self.get_block_result(Some(height)).await?;

        let txs = block
            .txs
            .into_iter()
            .zip(block_result.txs)
            .map(|(tx, tx_outcome)| SearchTxOutcome::new(height, tx, tx_outcome))
            .collect();

        Ok(BlockLogs::new(
            block.height,
            block.hash,
            txs,
            block_result.cronjobs,
        ))
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
