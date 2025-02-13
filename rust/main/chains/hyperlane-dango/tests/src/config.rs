use std::sync::LazyLock;

use grug::Coin;
use hyperlane_dango::{ConnectionConf, RpcConfig};

pub const DANGO_CONF_RPC: LazyLock<ConnectionConf> = LazyLock::new(|| ConnectionConf {
    provider_conf: hyperlane_dango::ProviderConf::Rpc(RpcConfig {
        url: "".to_string(),
        chain_id: "dango".to_string(),
    }),
    canonical_asset: todo!(),
    gas_price: Coin::new("uusdc", 0).unwrap(),
    gas_scale: 1.2,
    flat_gas_increase: 100_000,
    search_sleep_duration: 60,
    search_retry_attempts: 5,
});
