use {
    crate::HashConvertor,
    grug::{Coin, Hash256, Inner, Tx, TxOutcome},
};

pub struct SearchTxOutcome {
    pub tx: Tx,
    pub outcome: TxOutcome,
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

pub struct SimulateOutcome {
    pub outcome: TxOutcome,
    pub gas_adjusted: u64,
}
