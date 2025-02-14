use {
    dango_types::{account_factory::Username, config::AppConfig},
    ethers_prometheus::middleware::PrometheusMiddlewareConf,
    grug::{Addr, Coin, Defined, HexByteArray, MaybeDefined, Undefined},
    hyperlane_base::settings::{
        ChainConf, ChainConnectionConf, CoreContractAddresses, IndexSettings,
    },
    hyperlane_core::{HyperlaneDomain, KnownHyperlaneDomain, H256},
    hyperlane_dango::{
        ConnectionConf, DangoConvertor, DangoProvider, GraphQlConfig, ProviderConf, RpcConfig,
    },
    std::{collections::HashMap, num::NonZero, str::FromStr, sync::LazyLock},
};

const DANGO_DOMAIN: HyperlaneDomain = HyperlaneDomain::Known(KnownHyperlaneDomain::Dango);

const RPC_PROVIDER: LazyLock<RpcConfig> = LazyLock::new(|| RpcConfig {
    url: "".to_string(),
    chain_id: "dango".to_string(),
});

const GRAPHQL_PROVIDER: LazyLock<GraphQlConfig> = LazyLock::new(|| GraphQlConfig {});

pub fn build_connection_conf(provider_conf: ProviderConf) -> ConnectionConf {
    ConnectionConf {
        provider_conf,
        gas_price: Coin::new("uusdc", 0).unwrap(),
        gas_scale: 1.2,
        flat_gas_increase: 100_000,
        search_sleep_duration: 60,
        search_retry_attempts: 5,
    }
}

pub fn build_chain_conf(connection: ConnectionConf) -> ChainConf {
    ChainConf {
        domain: HyperlaneDomain::Known(KnownHyperlaneDomain::Dango),
        signer: Some(hyperlane_base::settings::SignerConf::Dango {
            username: Username::from_str("username").unwrap(),
            key: HexByteArray::from_inner([0; 32]),
            address: Addr::from_str("0xcf8c496fb3ff6abd98f2c2b735a0a148fed04b54").unwrap(),
        }),
        reorg_period: hyperlane_core::ReorgPeriod::Blocks(NonZero::new(3).unwrap()),
        addresses: CoreContractAddresses {
            mailbox: H256::from_str("mailbox").unwrap(),
            interchain_gas_paymaster: H256::from_str("igs").unwrap(),
            validator_announce: H256::from_str("validator_announce").unwrap(),
            merkle_tree_hook: H256::from_str("merkle_tree_hook").unwrap(),
        },
        connection: ChainConnectionConf::Dango(connection),
        metrics_conf: PrometheusMiddlewareConf {
            contracts: HashMap::new(),
            chain: None,
        },
        index: IndexSettings {
            from: 0,
            chunk_size: 10,
            mode: hyperlane_core::IndexMode::Block,
        },
    }
}

pub struct ChainConfBuilder<T = Undefined<CoreContractAddresses>, P = Undefined<ProviderConf>>
where
    T: MaybeDefined<CoreContractAddresses>,
    P: MaybeDefined<ProviderConf>,
{
    addresses: T,
    provider_conf: P,
}

impl ChainConfBuilder<Undefined<CoreContractAddresses>, Undefined<ProviderConf>> {
    pub fn new() -> Self {
        Self {
            addresses: Undefined::new(),
            provider_conf: Undefined::new(),
        }
    }
}

impl<T> ChainConfBuilder<T, Undefined<ProviderConf>>
where
    T: MaybeDefined<CoreContractAddresses>,
{
    pub fn with_provider_conf(
        self,
        provider_conf: ProviderConf,
    ) -> ChainConfBuilder<T, Defined<ProviderConf>> {
        ChainConfBuilder {
            addresses: self.addresses,
            provider_conf: Defined::new(provider_conf),
        }
    }

    pub fn with_default_rpc_provider(self) -> ChainConfBuilder<T, Defined<ProviderConf>> {
        ChainConfBuilder {
            addresses: self.addresses,
            provider_conf: Defined::new(ProviderConf::Rpc(RPC_PROVIDER.clone().to_owned())),
        }
    }

    pub fn with_default_graphql_provider(self) -> ChainConfBuilder<T, Defined<ProviderConf>> {
        ChainConfBuilder {
            addresses: self.addresses,
            provider_conf: Defined::new(ProviderConf::GraphQl(GRAPHQL_PROVIDER.clone().to_owned())),
        }
    }
}

impl<T> ChainConfBuilder<T, Defined<ProviderConf>>
where
    T: MaybeDefined<CoreContractAddresses>,
{
    pub async fn build(self) -> ChainConf {
        let connection = build_connection_conf(self.provider_conf.into_inner());

        let addresses = if let Some(addresses) = self.addresses.maybe_into_inner() {
            addresses
        } else {
            let provider = DangoProvider::from_config(&connection, DANGO_DOMAIN, None).unwrap();
            let res: AppConfig = provider.query_app_config().await.unwrap();
            let hyperlane_adddresses = res.addresses.hyperlane;

            CoreContractAddresses {
                mailbox: hyperlane_adddresses.mailbox.convert(),
                interchain_gas_paymaster: hyperlane_adddresses.fee.convert(),
                validator_announce: hyperlane_adddresses.va.convert(),
                merkle_tree_hook: hyperlane_adddresses.merkle.convert(),
            }
        };

        ChainConf {
            domain: DANGO_DOMAIN,
            signer: Some(hyperlane_base::settings::SignerConf::Dango {
                username: Username::from_str("username").unwrap(),
                key: HexByteArray::from_inner([0; 32]),
                address: Addr::from_str("0xcf8c496fb3ff6abd98f2c2b735a0a148fed04b54").unwrap(),
            }),
            reorg_period: hyperlane_core::ReorgPeriod::Blocks(NonZero::new(32).unwrap()),
            addresses,
            connection: ChainConnectionConf::Dango(connection),
            metrics_conf: PrometheusMiddlewareConf {
                contracts: HashMap::new(),
                chain: None,
            },
            index: IndexSettings {
                from: 0,
                chunk_size: 10,
                mode: hyperlane_core::IndexMode::Block,
            },
        }
    }
}
