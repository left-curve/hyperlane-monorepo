use {
    super::DangoMailbox,
    crate::{provider::DangoProvider, HashConvertor, SearchLog, TryHashConvertor},
    async_trait::async_trait,
    dango_hyperlane_types::mailbox,
    grug::Inner,
    hyperlane_core::{
        ChainResult, HyperlaneContract, HyperlaneMessage, Indexed, Indexer, LogMeta, H512,
    },
    std::ops::RangeInclusive,
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
        Ok(self
            .provider
            .fetch_logs(range)
            .await?
            .search_contract_log::<mailbox::Dispatch, _>(
                self.mailbox.address().try_convert()?,
                search_fn,
            )?)
    }

    /// Get the chain's latest block number that has reached finality
    async fn get_finalized_block_number(&self) -> ChainResult<u32> {
        Ok(self.provider.get_block(None).await?.height as u32)
    }

    /// Fetch list of logs emitted in a transaction with the given hash.
    async fn fetch_logs_by_tx_hash(
        &self,
        tx_hash: H512,
    ) -> ChainResult<Vec<(Indexed<HyperlaneMessage>, LogMeta)>> {
        Ok(self
            .provider
            .search_tx(tx_hash.try_convert()?)
            .await?
            .with_block_hash(&self.provider)
            .await?
            .search_contract_log(self.mailbox.address().try_convert()?, search_fn)?)
    }
}

fn search_fn(event: mailbox::Dispatch) -> Indexed<HyperlaneMessage> {
    HyperlaneMessage {
        version: event.0.version,
        nonce: event.0.nonce,
        origin: event.0.origin_domain,
        sender: event.0.sender.convert(),
        destination: event.0.destination_domain,
        recipient: event.0.recipient.convert(),
        body: event.0.body.into_inner(),
    }
    .into()
}
