use {
    super::DangoProvider,
    crate::{DangoConnectionConf, HashConvertor, TryHashConvertor},
    async_trait::async_trait,
    dango_types::auth::Metadata,
    grug::{Addr, Inner, JsonDeExt},
    hyperlane_core::{
        BlockInfo, ChainCommunicationError, ChainInfo, ChainResult, HyperlaneChain,
        HyperlaneDomain, HyperlaneProvider, TxnInfo, H256, H512, U256,
    },
    std::{fmt::Debug, ops::Deref, str::FromStr},
};

#[derive(Debug, Clone)]
pub struct HyperlaneDangoProvider<P>
where
    P: DangoProvider,
{
    domain: HyperlaneDomain,
    connection_conf: DangoConnectionConf,
    provider: P,
}

impl<P> HyperlaneChain for HyperlaneDangoProvider<P>
where
    P: DangoProvider + Clone + Send + Sync + Debug + 'static,
    ChainCommunicationError: From<P::Error>,
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
    ChainCommunicationError: From<P::Error>,
{
    /// Get block info for a given block height
    async fn get_block_by_height(&self, height: u64) -> ChainResult<BlockInfo> {
        let block = self.provider.get_block(Some(height)).await?;

        let hash = H256::from_slice(&block.hash);
        let time = block.timestamp.into_seconds();

        Ok(BlockInfo {
            hash: hash.to_owned(),
            timestamp: time as u64,
            number: block.height,
        })
    }

    /// Get txn info for a given txn hash
    async fn get_txn_by_hash(&self, hash: &H512) -> ChainResult<TxnInfo> {
        let tx = self.provider.search_tx(hash.try_convert()?).await?;

        let data: Metadata = tx.data.deserialize_json().map_err(|err| {
            ChainCommunicationError::from_other_str(&format!(
                "failed to deserialize data with hash {:?}: {:?}",
                hash, err
            ))
        })?;

        // let gas_price = self.calculate_gas_price(&hash, &tx)?;
        Ok(TxnInfo {
            hash: *hash,
            gas_limit: tx.gas_limit.into(),
            max_priority_fee_per_gas: None,
            max_fee_per_gas: None,
            // TODO: is this needed?
            gas_price: None,
            nonce: data.nonce.into(),
            sender: tx.sender.convert(),
            // TODO: is this needed?
            recipient: None,
            receipt: None,
            raw_input_data: None,
        })
    }

    /// Returns whether a contract exists at the provided address
    async fn is_contract(&self, address: &H256) -> ChainResult<bool> {
        Ok(self
            .provider
            .query_contract(address.try_convert()?)
            .await
            .is_ok())
    }

    /// Fetch the balance of the wallet address associated with the chain provider.
    async fn get_balance(&self, address: String) -> ChainResult<U256> {
        let address = Addr::from_str(&address)?;

        let balance = self
            .provider
            .query_balance(
                address,
                &self.connection_conf.get_canonical_asset().to_string(),
            )
            .await?;

        Ok(U256::from_big_endian(&balance.inner().to_be_bytes()))
    }

    /// Fetch metrics related to this chain
    async fn get_chain_metrics(&self) -> ChainResult<Option<ChainInfo>> {
        let block = self.provider.get_block(None).await?;
        return Ok(Some(ChainInfo {
            latest_block: BlockInfo {
                hash: block.hash.convert(),
                timestamp: block.timestamp.into_seconds() as u64,
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