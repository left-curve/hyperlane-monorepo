use {
    super::DangoMailbox,
    crate::{DangoConvertor, IntoDangoError, SearchLog, SearchTxOutcomeExt, TryDangoConvertor},
    async_trait::async_trait,
    dango_hyperlane_types::mailbox,
    grug::{Inner, QueryClientExt},
    hyperlane_core::{
        ChainResult, HyperlaneContract, HyperlaneMessage, Indexed, Indexer, LogMeta,
        SequenceAwareIndexer, H512,
    },
    std::ops::RangeInclusive,
};

#[async_trait]
impl Indexer<HyperlaneMessage> for DangoMailbox {
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
                self.address().try_convert()?,
                search_fn,
            )?)
    }

    /// Get the chain's latest block number that has reached finality
    async fn get_finalized_block_number(&self) -> ChainResult<u32> {
        Ok(self
            .provider
            .query_block(None)
            .await
            .into_dango_error()?
            .info
            .height as u32)
    }

    /// Fetch list of logs emitted in a transaction with the given hash.
    async fn fetch_logs_by_tx_hash(
        &self,
        tx_hash: H512,
    ) -> ChainResult<Vec<(Indexed<HyperlaneMessage>, LogMeta)>> {
        Ok(self
            .provider
            .search_tx(tx_hash.try_convert()?)
            .await
            .into_dango_error()?
            .with_block_hash(&self.provider)
            .await?
            .search_contract_log(self.address().try_convert()?, search_fn)?)
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

#[async_trait]
impl SequenceAwareIndexer<HyperlaneMessage> for DangoMailbox {
    async fn latest_sequence_count_and_tip(&self) -> ChainResult<(Option<u32>, u32)> {
        let last_height = self
            .provider
            .query_block(None)
            .await
            .into_dango_error()?
            .info
            .height;
        let nonce = self
            .provider
            .query_wasm_smart(
                self.address().try_convert()?,
                mailbox::QueryNonceRequest {},
                Some(last_height),
            )
            .await
            .into_dango_error()?;
        Ok((Some(nonce), last_height as u32))
    }
}
