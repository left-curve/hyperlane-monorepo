use grug::{Hash256, Tx, TxOutcome};

pub struct SearchTxOutcome {
    pub tx: Tx,
    pub outcome: TxOutcome,
}

pub struct BlockOutcome {
    pub hash: Hash256,
    pub height: u64,
    pub timestamp: u64,
    pub txs: Vec<Tx>,
}
