use {
    crate::{DangoResult, HashConvertor},
    grug::{
        Addr, CheckedContractEvent, Coin, CronOutcome, EventFilter, EventId, Hash256, HashExt,
        Inner, JsonDeExt, JsonSerExt, SearchEvent, StdResult, Tx, TxOutcome,
    },
    hyperlane_core::LogMeta,
    serde::de::DeserializeOwned,
};

pub struct SearchTxOutcome {
    pub tx: Tx,
    pub outcome: TxOutcome,
}

impl SearchTxOutcome {
    pub fn tx_hash(&self) -> DangoResult<Hash256> {
        Ok(self.tx.to_json_vec()?.hash256())
    }
}

impl SearchTxOutcome {
    pub fn into_hyperlane_outcome(
        self,
        hash: Hash256,
        gas_price: &Coin,
    ) -> hyperlane_core::TxOutcome {
        hyperlane_core::TxOutcome {
            transaction_id: hash.convert(),
            executed: self.outcome.result.is_ok(),
            gas_used: self.outcome.gas_used.into(),
            gas_price: gas_price.amount.inner().into(),
        }
    }
}

pub struct BlockOutcome {
    pub hash: Hash256,
    pub height: u64,
    pub timestamp: u64,
    pub txs: Vec<Tx>,
}

pub struct BlockResultOutcome {
    pub hash: Hash256,
    pub height: u64,
    pub txs: Vec<TxOutcome>,
    pub cronjobs: Vec<CronOutcome>,
}

pub struct SimulateOutcome {
    pub outcome: TxOutcome,
    pub gas_adjusted: u64,
}

pub struct BlockLogs {
    block_number: u64,
    block_hash: Hash256,
    txs: Vec<SearchTxOutcome>,
    crons: Vec<CronOutcome>,
}

impl BlockLogs {
    pub fn new(
        block_number: u64,
        block_hash: Hash256,
        txs: Vec<SearchTxOutcome>,
        crons: Vec<CronOutcome>,
    ) -> Self {
        Self {
            block_number,
            block_hash,
            txs,
            crons,
        }
    }
}

impl BlockLogs {
    pub fn search_contract_log<E>(self, contract: Addr) -> DangoResult<SearchLogResult<E>>
    where
        E: DeserializeOwned,
    {
        let closure = |filter: EventFilter<CheckedContractEvent>| {
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
            let res = closure(tx.outcome.events.search_event::<CheckedContractEvent>())?;
            if !res.is_empty() {
                outcome_tx.push((idx as u32, tx.tx, res));
            }
        }

        let mut cron_outcome = vec![];
        for (idx, cron) in self.crons.into_iter().enumerate() {
            let res = closure(cron.cron_event.search_event::<CheckedContractEvent>())?;
            if !res.is_empty() {
                cron_outcome.push((idx as u32, res));
            }
        }

        Ok(SearchLogResult {
            block_number: self.block_number,
            block_hash: self.block_hash,
            contract,
            tx: outcome_tx,
            cron: cron_outcome,
        })
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
