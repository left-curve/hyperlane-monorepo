use {
    super::DangoMailbox,
    crate::provider::DangoProvider,
    async_trait::async_trait,
    hyperlane_core::{ChainResult, HyperlaneMessage, Indexed, Indexer, LogMeta, H512},
    std::{io::Cursor, ops::RangeInclusive},
};

#[derive(Debug)]
pub struct DangoMailboxDispatchIndexer {
    mailbox: DangoMailbox,
    provider: Box<DangoProvider>,
}

#[async_trait]
impl Indexer<HyperlaneMessage> for DangoMailboxDispatchIndexer {
    /// Fetch list of logs between blocks `from` and `to`, inclusive.
    async fn fetch_logs_in_range(
        &self,
        range: RangeInclusive<u32>,
    ) -> ChainResult<Vec<(Indexed<HyperlaneMessage>, LogMeta)>> {
        todo!()
    }

    /// Get the chain's latest block number that has reached finality
    async fn get_finalized_block_number(&self) -> ChainResult<u32> {
        todo!()
    }

    /// Fetch list of logs emitted in a transaction with the given hash.
    async fn fetch_logs_by_tx_hash(
        &self,
        _tx_hash: H512,
    ) -> ChainResult<Vec<(Indexed<HyperlaneMessage>, LogMeta)>> {
        Ok(vec![])
    }
}
