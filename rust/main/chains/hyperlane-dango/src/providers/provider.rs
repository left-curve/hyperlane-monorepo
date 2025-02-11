use {
    super::{graphql::GraphQlProvider, DangoProvider},
    crate::{
        BlockOutcome, ConnectionConf, DangoSigner, HashConvertor, HyperlaneDangoResult,
        IntoHyperlaneDangoError, ProviderConf, SearchTxOutcome, TryHashConvertor,
    },
    anyhow::anyhow,
    async_trait::async_trait,
    dango_types::auth::Metadata,
    grug::{
        Addr, Coin, ContractInfo, Denom, Hash256, Inner, JsonDeExt, Message, Signer, SigningClient,
        Uint128,
    },
    hyperlane_core::{
        BlockInfo, ChainInfo, ChainResult, HyperlaneChain, HyperlaneDomain, HyperlaneProvider,
        TxnInfo, H256, H512, U256,
    },
    serde::{de::DeserializeOwned, Serialize},
    std::{
        fmt::Debug,
        ops::DerefMut,
        str::FromStr,
    },
};

#[derive(Debug, Clone)]
pub struct HyperlaneDangoProvider {
    pub domain: HyperlaneDomain,
    pub connection_conf: ConnectionConf,
    pub signer: Option<DangoSigner>,
    pub provider: ProviderWrapper,
}

impl HyperlaneDangoProvider {
    pub fn from_config(
        config: &ConnectionConf,
        domain: HyperlaneDomain,
        signer: Option<DangoSigner>,
    ) -> HyperlaneDangoResult<Self> {
        match &config.provider_conf {
            ProviderConf::Rpc(rpc_config) => Ok(HyperlaneDangoProvider {
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
}

impl HyperlaneChain for HyperlaneDangoProvider {
    fn domain(&self) -> &HyperlaneDomain {
        &self.domain
    }

    fn provider(&self) -> Box<dyn HyperlaneProvider> {
        Box::new(self.clone())
    }
}

#[async_trait]
impl HyperlaneProvider for HyperlaneDangoProvider {
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

impl HyperlaneDangoProvider {
    pub async fn get_block(&self, height: Option<u64>) -> HyperlaneDangoResult<BlockOutcome> {
        self.provider.get_block(height).await
    }

    /// Get transaction info for a given transaction hash.
    pub async fn search_tx(&self, hash: Hash256) -> HyperlaneDangoResult<SearchTxOutcome> {
        self.provider.search_tx(hash).await
    }

    /// Get the balance of an address for a given denom.
    pub async fn balance(&self, addr: Addr, denom: Denom) -> HyperlaneDangoResult<Uint128> {
        self.provider.balance(addr, denom).await
    }

    /// Get the contract info for a given contract address.
    pub async fn contract_info(&self, addr: Addr) -> HyperlaneDangoResult<ContractInfo> {
        self.provider.contract_info(addr).await
    }

    /// Query a wasm smart contract.
    pub async fn query_wasm_smart<M, R>(
        &self,
        contract: Addr,
        msg: &M,
        height: Option<u64>,
    ) -> HyperlaneDangoResult<R>
    where
        M: Serialize + Send + Sync,
        R: DeserializeOwned,
    {
        self.provider.query_wasm_smart(contract, msg, height).await
    }

    /// Sign and broadcast a message.
    pub async fn send_message(&self, msg: Message) -> HyperlaneDangoResult<Hash256> {
        let signer = self
            .signer
            .clone()
            .ok_or(anyhow!("can't use send_message if signer is not specified"))?;

        // get nonce:

        


        let response = self
            .provider
            .send_message(signer.write().await.deref_mut(), msg)
            .await?;

        Ok(response)
    }
}

#[derive(Debug, Clone)]
pub enum ProviderWrapper {
    Rpc(SigningClient),
    GraphQl(GraphQlProvider),
}

#[async_trait]
impl DangoProvider for ProviderWrapper {
    /// Get block info for a given block height. If block height is None, return the latest block.
    async fn get_block(&self, height: Option<u64>) -> HyperlaneDangoResult<BlockOutcome> {
        match self {
            ProviderWrapper::Rpc(provider) => provider.get_block(height).await,
            ProviderWrapper::GraphQl(provider) => provider.get_block(height).await,
        }
    }

    /// Get transaction info for a given transaction hash.
    async fn search_tx(&self, hash: Hash256) -> HyperlaneDangoResult<SearchTxOutcome> {
        match self {
            ProviderWrapper::Rpc(provider) => provider.search_tx(hash).await,
            ProviderWrapper::GraphQl(provider) => provider.search_tx(hash).await,
        }
    }

    /// Get the balance of an address for a given denom.
    async fn balance(&self, addr: Addr, denom: Denom) -> HyperlaneDangoResult<Uint128> {
        match self {
            ProviderWrapper::Rpc(provider) => provider.balance(addr, denom).await,
            ProviderWrapper::GraphQl(provider) => provider.balance(addr, denom).await,
        }
    }

    /// Get the contract info for a given contract address.
    async fn contract_info(&self, addr: Addr) -> HyperlaneDangoResult<ContractInfo> {
        match self {
            ProviderWrapper::Rpc(provider) => provider.contract_info(addr).await,
            ProviderWrapper::GraphQl(provider) => provider.contract_info(addr).await,
        }
    }

    /// Query a wasm smart contract.
    async fn query_wasm_smart<M, R>(
        &self,
        contract: Addr,
        msg: &M,
        height: Option<u64>,
    ) -> HyperlaneDangoResult<R>
    where
        M: Serialize + Send + Sync,
        R: DeserializeOwned,
    {
        match self {
            ProviderWrapper::Rpc(provider) => {
                provider.query_wasm_smart(contract, msg, height).await
            }
            ProviderWrapper::GraphQl(provider) => {
                provider.query_wasm_smart(contract, msg, height).await
            }
        }
    }

    /// Sign and broadcast a message.
    async fn send_message<S>(&self, signer: &mut S, msg: Message) -> HyperlaneDangoResult<Hash256>
    where
        S: Signer + Send + Sync,
    {
        match self {
            ProviderWrapper::Rpc(provider) => {
                DangoProvider::send_message(provider, signer, msg).await
            }
            ProviderWrapper::GraphQl(provider) => provider.send_message(signer, msg).await,
        }
    }
}
