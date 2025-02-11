use {
    crate::{provider::HyperlaneDangoProvider, DangoSigner, DangoResult},
    grug::{Coin, Denom},
    hyperlane_core::{HyperlaneDomain, HyperlaneProvider},
};

/// Dango connection configuration
#[derive(Debug, Clone)]
pub struct ConnectionConf {
    /// Provider configuration
    pub provider_conf: ProviderConf,
    /// Canonical Assets Denom
    pub canonical_asset: Denom,
    // Gas price
    pub gas_price: Coin,
}

#[derive(Debug, Clone)]
pub enum ProviderConf {
    Rpc(RpcConfig),
    GraphQl(GraphQlConfig),
}

#[derive(Debug, Clone)]
pub struct RpcConfig {
    pub url: String,
    pub chain_id: String,
}
#[derive(Debug, Clone)]
pub struct GraphQlConfig {}

impl ConnectionConf {
    /// Build a provider.
    pub fn build_provider(
        &self,
        domain: HyperlaneDomain,
        signer: Option<DangoSigner>,
    ) -> DangoResult<Box<dyn HyperlaneProvider>> {
        Ok(Box::new(HyperlaneDangoProvider::from_config(
            &self,
            domain,
            signer,
        )?))
    }

    /// Returns canonical asset.
    pub fn get_canonical_asset(&self) -> &Denom {
        &self.canonical_asset
    }

    /// Returns gas price.
    pub fn get_gas_price(&self) -> &Coin {
        &self.gas_price
    }
}
