use grug::{Coin, Denom, Hash256, Tx, TxOutcome};

/// Cosmos connection configuration
#[derive(Debug, Clone)]
pub struct DangoConnectionConf {
    /// The RPC url to connect to
    // rpc_urls: Vec<Url>,
    /// The chain ID
    // chain_id: String,
    /// Canonical Assets Denom
    canonical_asset: Denom,
    // Gas price
    gas_price: Coin,
}

impl DangoConnectionConf {
    /// Returns canonical asset.
    pub fn get_canonical_asset(&self) -> &Denom {
        &self.canonical_asset
    }

    /// Returns gas price.
    pub fn get_gas_price(&self) -> &Coin {
        &self.gas_price
    }
}

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
