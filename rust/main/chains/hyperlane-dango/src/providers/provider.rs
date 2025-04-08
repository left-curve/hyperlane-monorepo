use {
    super::{ClientRef, Wrapper},
    crate::{
        BlockLogs, ConnectionConf, DangoConvertor, DangoResult, DangoSigner, ExecutionBlock,
        IntoDangoError, ProviderConf, TryDangoConvertor,
    },
    anyhow::anyhow,
    async_trait::async_trait,
    dango_types::{account::spot, auth::Metadata},
    futures_util::future::try_join_all,
    grug::{
        Addr, BroadcastClientExt, Defined, GasOption, Hash256, Inner, JsonDeExt, Message, NonEmpty,
        QueryClientExt, SearchTxOutcome, Signer, TendermintRpcClient,
    },
    grug_indexer_client::HttpClient,
    hyperlane_core::{
        BlockInfo, ChainInfo, ChainResult, HyperlaneChain, HyperlaneDomain, HyperlaneProvider,
        ReorgPeriod, TxnInfo, H256, H512, U256,
    },
    std::{
        ops::{Deref, DerefMut, RangeInclusive},
        str::FromStr,
        sync::Arc,
    },
};

#[derive(Debug, Clone)]
pub struct DangoProvider {
    pub domain: HyperlaneDomain,
    pub connection_conf: ConnectionConf,
    pub signer: Option<DangoSigner>,
    client: Wrapper,
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
        let block = self.query_block(Some(height)).await.into_dango_error()?;

        Ok(BlockInfo {
            hash: block.info.hash.convert(),
            timestamp: block.info.timestamp.into_seconds() as u64,
            number: block.info.height,
        })
    }

    /// Get txn info for a given txn hash
    async fn get_txn_by_hash(&self, hash: &H512) -> ChainResult<TxnInfo> {
        let tx = self
            .search_tx(hash.try_convert()?)
            .await
            .into_dango_error()?;

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
        match self
            .client
            .query_contract(address.try_convert()?, None)
            .await
        {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }

    /// Fetch the balance of the wallet address associated with the chain provider.
    async fn get_balance(&self, address: String) -> ChainResult<U256> {
        let address = Addr::from_str(&address).into_dango_error()?;

        let balance = self
            .query_balance(address, self.connection_conf.gas_price.denom.clone(), None)
            .await
            .into_dango_error()?;

        Ok(balance.into_inner().into())
    }

    /// Fetch metrics related to this chain
    async fn get_chain_metrics(&self) -> ChainResult<Option<ChainInfo>> {
        let block = self.query_block(None).await.into_dango_error()?;
        return Ok(Some(ChainInfo {
            latest_block: BlockInfo {
                hash: block.info.hash.convert(),
                timestamp: block.info.timestamp.into_seconds() as u64,
                number: block.info.height,
            },
            min_gas_price: None,
        }));
    }
}

impl Deref for DangoProvider {
    type Target = Wrapper;

    fn deref(&self) -> &Self::Target {
        &self.client
    }
}

impl DangoProvider {
    pub fn from_config(
        config: &ConnectionConf,
        domain: HyperlaneDomain,
        signer: Option<DangoSigner>,
    ) -> DangoResult<Self> {
        let client = match &config.provider_conf {
            ProviderConf::Rpc(_) => {
                let rpc = TendermintRpcClient::new(
                    config
                        .rpcs
                        .first()
                        .ok_or(anyhow!("rpcs is empty"))?
                        .as_str(),
                )?;

                Arc::new(rpc) as Arc<dyn ClientRef<anyhow::Error>>
            }
            ProviderConf::GraphQl(config) => {
                let graphql = HttpClient::new(config.url.as_str());
                Arc::new(graphql) as Arc<dyn ClientRef<anyhow::Error>>
            }
        };

        Ok(DangoProvider {
            domain,
            connection_conf: config.clone(),
            signer,
            client: Wrapper { client },
        })
    }

    pub async fn fetch_logs(&self, range: RangeInclusive<u32>) -> DangoResult<Vec<BlockLogs>> {
        let tasks = range
            .into_iter()
            .map(|i| async move { self.get_block_logs(i as u64).await })
            .collect::<Vec<_>>();

        try_join_all(tasks).await
    }

    async fn get_block_logs(&self, height: u64) -> DangoResult<BlockLogs> {
        let block = self.query_block(Some(height)).await?;
        let block_result = self.query_block_outcome(Some(height)).await?;

        let txs = block
            .txs
            .into_iter()
            .zip(block_result.tx_outcomes)
            .enumerate()
            .map(|(idx, ((tx, tx_hash), tx_outcome))| SearchTxOutcome {
                hash: tx_hash,
                height,
                index: idx as u32,
                tx,
                outcome: tx_outcome,
            })
            .collect();

        Ok(BlockLogs::new(
            block.info.height,
            block.info.hash,
            txs,
            block_result.cron_outcomes,
        ))
    }

    fn signer(&self) -> DangoResult<DangoSigner> {
        Ok(self
            .signer
            .clone()
            .ok_or(anyhow!("can't use send_message if signer is not specified"))?)
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

    /// Estimate the costs of a message.
    pub async fn estimate_costs(
        &self,
        msg: Message,
    ) -> DangoResult<hyperlane_core::TxCostEstimate> {
        let tx = self.signer()?.read().await.deref().unsigned_transaction(
            NonEmpty::new_unchecked(vec![msg]),
            &self.connection_conf.chain_id,
        )?;
        let outcome = self.simulate(tx).await?;

        Ok(hyperlane_core::TxCostEstimate {
            gas_limit: ((outcome.gas_used as f64 * self.connection_conf.gas_scale) as u64
                + self.connection_conf.flat_gas_increase)
                .into(),
            gas_price: self.connection_conf.gas_price.amount.inner().into(),
            l2_gas_limit: None,
        })
    }

    /// Sign and broadcast a message.
    pub async fn send_message_and_find(
        &self,
        msg: Message,
        gas_limit: Option<u64>,
    ) -> DangoResult<hyperlane_core::TxOutcome> {
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

        let hash = self
            .send_message(
                signer.write().await.deref_mut(),
                msg,
                gas,
                &self.connection_conf.chain_id,
            )
            .await?
            .tx_hash;

        let outcome = self.search_tx_loop(hash).await?;

        return Ok(hyperlane_core::TxOutcome {
            transaction_id: outcome.hash.convert(),
            executed: outcome.outcome.result.is_ok(),
            gas_used: outcome.outcome.gas_used.into(),
            gas_price: self.connection_conf.gas_price.amount.inner().into(),
        });
    }

    // // Utility

    /// Get the block height for a given reorg period.
    pub async fn get_block_height_by_reorg_period(
        &self,
        reorg_period: ReorgPeriod,
    ) -> DangoResult<Option<u64>> {
        let block_height = match reorg_period {
            ReorgPeriod::Blocks(blocks) => {
                let last_block = self.query_block(None).await?;
                let block_height = last_block.info.height - blocks.get() as u64;
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
}
