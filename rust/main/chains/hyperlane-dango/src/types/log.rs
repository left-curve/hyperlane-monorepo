use {
    super::SearchTxOutcome,
    crate::{DangoResult, HashConvertor},
    grug::{
        Addr, Addressable, CheckedContractEvent, CronOutcome, Defined, EventFilter, EventId,
        Hash256, HashExt, JsonDeExt, SearchEvent, StdResult, Tx, Undefined,
    },
    hyperlane_core::LogMeta,
    serde::de::DeserializeOwned,
};

pub struct BlockLogs<H = Undefined<Hash256>> {
    pub(crate) block_height: u64,
    pub(crate) block_hash: Hash256,
    pub(crate) txs: Vec<SearchTxOutcome<H>>,
    pub(crate) crons: Vec<CronOutcome>,
}

impl BlockLogs {
    pub fn new(
        block_number: u64,
        block_hash: Hash256,
        txs: Vec<SearchTxOutcome>,
        crons: Vec<CronOutcome>,
    ) -> Self {
        Self {
            block_height: block_number,
            block_hash,
            txs,
            crons,
        }
    }
}

pub struct SearchLogResult<E> {
    pub(crate) block_number: u64,
    pub(crate) block_hash: Hash256,
    pub(crate) contract: Addr,
    pub(crate) tx: Vec<(u32, Tx, Vec<(E, EventId)>)>,
    pub(crate) cron: Vec<(u32, Vec<(E, EventId)>)>,
}

impl<E> SearchLogResult<E> {
    pub fn finalize<R, F: Fn(E) -> R>(self, closure: F) -> Vec<(R, LogMeta)> {
        let block_number = self.block_number;
        let block_hash = self.block_hash.convert();
        let address = self.contract.convert();

        let mut output = vec![];

        for (idx, tx, logs) in self.tx {
            for (log, log_index) in logs {
                let transaction_id = tx.tx_hash().unwrap().convert();
                let meta = LogMeta {
                    address,
                    block_number,
                    block_hash,
                    transaction_id,
                    transaction_index: idx as u64,
                    log_index: log_index.event_index.into(),
                };

                output.push((closure(log), meta));
            }
        }

        for (idx, logs) in self.cron {
            for (log, log_index) in logs {
                let cron_id = cron_hash(self.block_number, idx);
                let meta = LogMeta {
                    address,
                    block_number,
                    block_hash,
                    transaction_id: cron_id.convert(),
                    transaction_index: idx as u64,
                    log_index: log_index.event_index.into(),
                };

                output.push((closure(log), meta));
            }
        }

        output
    }
}

fn cron_hash(block: u64, cron_id: u32) -> Hash256 {
    let mut bytes = [0; 8 + 4];
    bytes[..8].copy_from_slice(&block.to_be_bytes());
    bytes[8..].copy_from_slice(&cron_id.to_be_bytes());
    bytes.hash256()
}

pub trait SearchLog {
    fn search_contract_log<E, R>(
        self,
        contract: Addr,
        closure: fn(E) -> R,
    ) -> DangoResult<Vec<(R, LogMeta)>>
    where
        E: DeserializeOwned;
}

impl<H> SearchLog for BlockLogs<H> {
    fn search_contract_log<E, R>(
        self,
        contract: Addr,
        return_closure: fn(E) -> R,
    ) -> DangoResult<Vec<(R, LogMeta)>>
    where
        E: DeserializeOwned,
    {
        let filter_closure = |filter: EventFilter<CheckedContractEvent>| {
            filter
                .with_commitment_status(grug::FlatCommitmentStatus::Committed)
                .with_predicate(move |e| e.contract == contract)
                .take()
                .all()
                .into_iter()
                .map(|e| e.event.data.deserialize_json().map(|c| (c, e.id)))
                .collect::<StdResult<Vec<(E, _)>>>()
        };

        let mut outcome_tx = vec![];
        for (idx, tx) in self.txs.into_iter().enumerate() {
            let res = filter_closure(tx.outcome.events.search_event::<CheckedContractEvent>())?;
            if !res.is_empty() {
                outcome_tx.push((idx as u32, tx.tx, res));
            }
        }

        let mut cron_outcome = vec![];
        for (idx, cron) in self.crons.into_iter().enumerate() {
            let res = filter_closure(cron.cron_event.search_event::<CheckedContractEvent>())?;
            if !res.is_empty() {
                cron_outcome.push((idx as u32, res));
            }
        }

        let search_log_result = SearchLogResult {
            block_number: self.block_height,
            block_hash: self.block_hash,
            contract,
            tx: outcome_tx,
            cron: cron_outcome,
        };

        Ok(search_log_result.finalize(return_closure))
    }
}

impl<H> SearchLog for Vec<BlockLogs<H>> {
    fn search_contract_log<E, R>(
        self,
        contract: Addr,
        closure: fn(E) -> R,
    ) -> DangoResult<Vec<(R, LogMeta)>>
    where
        E: DeserializeOwned,
    {
        self.into_iter()
            .try_fold(vec![], |mut buff, log| -> DangoResult<_> {
                buff.extend(log.search_contract_log::<E, R>(contract.address(), closure)?);

                Ok(buff)
            })
    }
}

impl SearchLog for SearchTxOutcome<Defined<Hash256>> {
    fn search_contract_log<E, R>(
        self,
        contract: Addr,
        closure: fn(E) -> R,
    ) -> DangoResult<Vec<(R, LogMeta)>>
    where
        E: DeserializeOwned,
    {
        BlockLogs {
            block_height: self.block_height,
            block_hash: self.block_hash.into_inner(),
            txs: vec![self],
            crons: vec![],
        }
        .search_contract_log(contract, closure)
    }
}
