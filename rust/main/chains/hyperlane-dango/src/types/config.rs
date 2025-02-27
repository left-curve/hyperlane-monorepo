use {
    crate::{DangoProvider, DangoResult, DangoSigner},
    grug::Coin,
    hyperlane_core::{config::OperationBatchConfig, HyperlaneDomain, HyperlaneProvider},
    serde::{Deserialize, Serialize},
    url::Url,
};

/// Dango connection configuration
#[derive(Debug, Clone)]
pub struct ConnectionConf {
    /// Provider configuration
    pub provider_conf: ProviderConf,
    // Gas price
    pub gas_price: Coin,
    /// Gas scale
    pub gas_scale: f64,
    /// Flat gas increase
    pub flat_gas_increase: u64,
    /// Search sleep duration in seconds
    pub search_sleep_duration: u64,
    /// Search retry attempts
    pub search_retry_attempts: u64,
    pub chain_id: String,
    pub rpcs: Vec<Url>,
    pub operation_batch: OperationBatchConfig
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderConf {
    Rpc(RpcConfig),
    GraphQl(GraphQlConfig),
}

#[derive(Debug, Clone, Serialize, Deserialize)]

pub struct RpcConfig {}

#[derive(Debug, Clone, Serialize, Deserialize)]

pub struct GraphQlConfig {}

impl ConnectionConf {
    /// Build a provider.
    pub fn build_provider(
        &self,
        domain: HyperlaneDomain,
        signer: Option<DangoSigner>,
    ) -> DangoResult<Box<dyn HyperlaneProvider>> {
        Ok(Box::new(DangoProvider::from_config(&self, domain, signer)?))
    }
}
