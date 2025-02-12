use {
    super::DangoMerkleTreeHook,
    crate::{provider::DangoProvider, HashConvertor, TryHashConvertor},
    async_trait::async_trait,
    dango_hyperlane_types::hooks::merkle,
    hyperlane_core::{
        ChainResult, HyperlaneContract, Indexed, Indexer, LogMeta, MerkleTreeHook,
        MerkleTreeInsertion, ReorgPeriod, SequenceAwareIndexer, H512,
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
        self.provider.fetch_logs(range).await?.into_iter().try_fold(
            vec![],
            |mut buff, log| -> ChainResult<_> {
                buff.extend(
                    log.search_contract_log::<merkle::PostDispatch>(
                        self.merkle_tree.address().try_convert()?,
                    )?
                    .finalize(|event| {
                        MerkleTreeInsertion::new(event.index as u32, event.message_id.convert())
                            .into()
                    }),
                );

                Ok(buff)
            },
        )
    }

    /// Get the chain's latest block number that has reached finality
    async fn get_finalized_block_number(&self) -> ChainResult<u32> {
        Ok(self.provider.get_block(None).await?.height as u32)
    }

    /// Fetch list of logs emitted in a transaction with the given hash.
    async fn fetch_logs_by_tx_hash(
        &self,
        _tx_hash: H512,
    ) -> ChainResult<Vec<(Indexed<MerkleTreeInsertion>, LogMeta)>> {
        Ok(vec![])
    }
}

#[async_trait]
impl SequenceAwareIndexer<MerkleTreeInsertion> for DangoMerkleTreeIndexer {
    /// Return the latest finalized sequence (if any) and block number
    async fn latest_sequence_count_and_tip(&self) -> ChainResult<(Option<u32>, u32)> {
        let block = self.provider.get_block(None).await?;
        // TODO: This is not 100% correct.
        // Its better to query che contract a this specific block height.
        let sequence = self.merkle_tree.count(&ReorgPeriod::None).await?;
        return Ok((Some(sequence), block.height as u32));
    }
}
