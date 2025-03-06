use {
    super::DangoMerkleTree,
    crate::{DangoConvertor, ExecutionBlock, SearchLog, TryDangoConvertor},
    async_trait::async_trait,
    dango_hyperlane_types::hooks::merkle,
    hyperlane_core::{
        ChainResult, HyperlaneContract, Indexed, Indexer, LogMeta, MerkleTreeInsertion,
        SequenceAwareIndexer, H512,
    },
    std::ops::RangeInclusive,
};

#[async_trait]
impl Indexer<MerkleTreeInsertion> for DangoMerkleTree {
    /// Fetch list of logs between blocks `from` and `to`, inclusive.
    async fn fetch_logs_in_range(
        &self,
        range: RangeInclusive<u32>,
    ) -> ChainResult<Vec<(Indexed<MerkleTreeInsertion>, LogMeta)>> {
        Ok(self
            .provider
            .fetch_logs(range)
            .await?
            .search_contract_log(self.address().try_convert()?, search_fn)?)
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
            .search_contract_log(self.address().try_convert()?, search_fn)?)
    }
}

fn search_fn(event: merkle::PostDispatch) -> Indexed<MerkleTreeInsertion> {
    MerkleTreeInsertion::new(event.index as u32, event.message_id.convert()).into()
}

#[async_trait]
impl SequenceAwareIndexer<MerkleTreeInsertion> for DangoMerkleTree {
    /// Return the latest finalized sequence (if any) and block number
    async fn latest_sequence_count_and_tip(&self) -> ChainResult<(Option<u32>, u32)> {
        let last_height = self.provider.get_block(None).await?.height;
        let dango_tree = self
            .dango_tree(ExecutionBlock::Defined(last_height))
            .await?;

        return Ok((Some(dango_tree.count as u32), last_height as u32));
    }
}
