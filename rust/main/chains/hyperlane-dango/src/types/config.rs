use {
    crate::{provider::HyperlaneDangoProvider, HyperlaneDangoResult},
    grug::{Coin, Denom, SigningClient},
    hyperlane_core::{HyperlaneDomain, HyperlaneProvider},
};

/// Cosmos connection configuration
#[derive(Debug, Clone)]
pub struct ConnectionConf {
    /// Provider configuration
    provider_conf: ProviderConf,
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
    ) -> HyperlaneDangoResult<Box<dyn HyperlaneProvider>> {
        match &self.provider_conf {
            ProviderConf::Rpc(config) => Ok(Box::new(HyperlaneDangoProvider {
                domain,
                connection_conf: self.clone(),
                provider: SigningClient::connect(config.chain_id.clone(), config.url.as_str())?,
            })),
            // TODO: DANGO
            ProviderConf::GraphQl(_) => unimplemented!(),
        }
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
