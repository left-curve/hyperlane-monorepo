use {
    super::DangoMailbox,
    crate::{provider::DangoProvider, HashConvertor, TryHashConvertor},
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
        self.provider.fetch_logs(range).await?.into_iter().try_fold(
            vec![],
            |mut buff, log| -> ChainResult<_> {
                buff.extend(
                    log.search_contract_log::<mailbox::Dispatch>(
                        self.mailbox.address().try_convert()?,
                    )?
                    .finalize(|event| {
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
                    }),
                );

                Ok(buff)
            },
        )
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
