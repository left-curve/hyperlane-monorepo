use {
    super::DangoMerkleTreeHook,
    crate::{provider::DangoProvider, HashConvertor, SearchLog, TryHashConvertor},
    async_trait::async_trait,
    dango_hyperlane_types::hooks::merkle,
    hyperlane_core::{
        ChainResult, HyperlaneContract, Indexed, Indexer, LogMeta, MerkleTreeInsertion, H512,
    },
    std::ops::RangeInclusive,
};

#[derive(Debug)]
pub struct DangoMerkleTreeIndexer {
    pub merkle_tree: DangoMerkleTreeHook,
    pub provider: DangoProvider,
}

#[async_trait]
impl Indexer<MerkleTreeInsertion> for DangoMerkleTreeIndexer {
    /// Fetch list of logs between blocks `from` and `to`, inclusive.
    async fn fetch_logs_in_range(
        &self,
        range: RangeInclusive<u32>,
    ) -> ChainResult<Vec<(Indexed<MerkleTreeInsertion>, LogMeta)>> {
        Ok(self
            .provider
            .fetch_logs(range)
            .await?
            .search_contract_log(self.merkle_tree.address().try_convert()?, search_fn)?)
    }

    /// Get the chain's latest block number that has reached finality
    async fn get_finalized_block_number(&self) -> ChainResult<u32> {
        Ok(self.provider.get_block(None).await?.height as u32)
    }

    /// Fetch list of logs emitted in a transaction with the given hash.
    async fn fetch_logs_by_tx_hash(
        &self,
        tx_hash: H512,
    ) -> ChainResult<Vec<(Indexed<MerkleTreeInsertion>, LogMeta)>> {
        Ok(self
            .provider
            .search_tx(tx_hash.try_convert()?)
            .await?
            .with_block_hash(&self.provider)
            .await?
            .search_contract_log(self.merkle_tree.address().try_convert()?, search_fn)?)
    }
}

fn search_fn(event: merkle::PostDispatch) -> Indexed<MerkleTreeInsertion> {
    MerkleTreeInsertion::new(event.index as u32, event.message_id.convert()).into()
}
