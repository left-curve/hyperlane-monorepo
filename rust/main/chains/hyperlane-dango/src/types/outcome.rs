use {
    crate::{provider::DangoProvider, DangoResult, HashConvertor},
    grug::{Coin, CronOutcome, Defined, Hash256, Inner, Tx, TxOutcome, Undefined},
};

pub struct SearchTxOutcome<H = Undefined<Hash256>> {
    pub block_height: u64,
    pub block_hash: H,
    pub tx: Tx,
    pub outcome: TxOutcome,
}

impl SearchTxOutcome {
    pub fn new(block_height: u64, tx: Tx, outcome: TxOutcome) -> Self {
        Self {
            block_height,
            block_hash: Undefined::new(),
            tx,
            outcome,
        }
    }

    pub async fn with_block_hash(
        self,
        provider: &DangoProvider,
    ) -> DangoResult<SearchTxOutcome<Defined<Hash256>>> {
        let hash = provider.get_block(Some(self.block_height)).await?.hash;

        Ok(SearchTxOutcome {
            block_height: self.block_height,
            block_hash: Defined::new(hash),
            tx: self.tx,
            outcome: self.outcome,
        })
    }
}

impl<H> SearchTxOutcome<H> {
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
