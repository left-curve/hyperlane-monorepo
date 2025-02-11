use {
    super::DangoProvider,
    crate::{ConnectionConf, HashConvertor, IntoHyperlaneDangoError, TryHashConvertor},
    async_trait::async_trait,
    dango_types::auth::Metadata,
    grug::{Addr, Coin, Inner, JsonDeExt},
    hyperlane_core::{
        BlockInfo, ChainInfo, ChainResult, HyperlaneChain, HyperlaneDomain, HyperlaneProvider,
        TxnInfo, H256, H512, U256,
    },
    std::{fmt::Debug, ops::Deref, str::FromStr},
};

#[derive(Debug, Clone)]
pub struct HyperlaneDangoProvider<P>
where
    P: DangoProvider,
{
    pub domain: HyperlaneDomain,
    pub connection_conf: ConnectionConf,
    pub provider: P,
}

impl<P> HyperlaneChain for HyperlaneDangoProvider<P>
where
    P: DangoProvider + Clone + Send + Sync + Debug + 'static,
{
    fn domain(&self) -> &HyperlaneDomain {
        &self.domain
    }

    fn provider(&self) -> Box<dyn HyperlaneProvider> {
        Box::new(self.clone())
    }
}
#[async_trait]
impl<P> HyperlaneProvider for HyperlaneDangoProvider<P>
where
    P: DangoProvider + Clone + Send + Sync + Debug + 'static,
{
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

impl<P> Deref for HyperlaneDangoProvider<P>
where
    P: DangoProvider,
{
    type Target = P;

    fn deref(&self) -> &Self::Target {
        &self.provider
    }
}

impl<P> HyperlaneDangoProvider<P>
where
    P: DangoProvider,
{
    pub fn get_gas_price(&self) -> &Coin {
        self.connection_conf.get_gas_price()
    }
}
