use {
    grug::Denom,
    url::Url,
};

/// Cosmos connection configuration
#[derive(Debug, Clone)]
pub struct DangoConnectionConf {
    /// The RPC url to connect to
    rpc_urls: Vec<Url>,
    /// The chain ID
    chain_id: String,
    /// Canonical Assets Denom
    canonical_asset: Denom,
    // gas_price: RawCosmosAmount,
}

impl DangoConnectionConf {

    /// Returns canonical asset.
    pub fn get_canonical_asset(&self) -> Denom {
        self.canonical_asset.clone()
    }
}
