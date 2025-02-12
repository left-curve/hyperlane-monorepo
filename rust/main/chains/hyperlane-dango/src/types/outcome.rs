use {
    crate::{DangoResult, HashConvertor},
    grug::{Coin, CronOutcome, Hash256, HashExt, Inner, JsonSerExt, Tx, TxOutcome},
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
