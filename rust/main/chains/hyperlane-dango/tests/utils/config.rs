use {
    dango_types::{account_factory::Username, config::AppConfig},
    ethers_prometheus::middleware::PrometheusMiddlewareConf,
    grug::{Addr, Coin, Defined, HexByteArray, MaybeDefined, Undefined},
    hyperlane_base::{
        settings::{
            ChainConf, ChainConnectionConf, CoreContractAddresses, IndexSettings, SignerConf,
        },
        CoreMetrics,
    },
    hyperlane_core::{HyperlaneDomain, KnownHyperlaneDomain, ReorgPeriod, H256},
    hyperlane_dango::{
        ConnectionConf, DangoConvertor, DangoProvider, DangoSigner, GraphQlConfig, ProviderConf,
        RpcConfig,
    },
    std::{collections::HashMap, num::NonZero, str::FromStr, sync::LazyLock},
    url::Url,
};

pub const DANGO_DOMAIN: HyperlaneDomain = HyperlaneDomain::Known(KnownHyperlaneDomain::Dango);

pub const EMPTY_METRICS: LazyLock<CoreMetrics> =
    LazyLock::new(|| CoreMetrics::new("dango", 9090, prometheus::Registry::new()).unwrap());

const CHAIN_ID: &str = "dango";
const URL: &str = "http://localhost:26657";

const GRAPHQL_PROVIDER: LazyLock<GraphQlConfig> = LazyLock::new(|| GraphQlConfig {});

pub fn build_connection_conf(provider_conf: ProviderConf) -> ConnectionConf {
    ConnectionConf {
        provider_conf,
        gas_price: Coin::new("uusdc", 0).unwrap(),
        gas_scale: 1.2,
        flat_gas_increase: 100_000,
        search_sleep_duration: 2,
        search_retry_attempts: 5,
        chain_id: CHAIN_ID.to_owned(),
        rpcs: vec![Url::parse(URL).unwrap()],
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

pub struct ChainConfBuilder<T, P, S>
where
    T: MaybeDefined<CoreContractAddresses>,
    P: MaybeDefined<ProviderConf>,
    S: MaybeDefined<SignerConf>,
{
    addresses: T,
    provider_conf: P,
    signer: S,
}

impl
    ChainConfBuilder<
        Undefined<CoreContractAddresses>,
        Undefined<ProviderConf>,
        Undefined<SignerConf>,
    >
{
    pub fn new() -> Self {
        Self {
            addresses: Undefined::new(),
            provider_conf: Undefined::new(),
            signer: Undefined::new(),
        }
    }
}

impl<T, S> ChainConfBuilder<T, Undefined<ProviderConf>, S>
where
    T: MaybeDefined<CoreContractAddresses>,
    S: MaybeDefined<SignerConf>,
{
    pub fn with_provider_conf(
        self,
        provider_conf: ProviderConf,
    ) -> ChainConfBuilder<T, Defined<ProviderConf>, S> {
        ChainConfBuilder {
            addresses: self.addresses,
            provider_conf: Defined::new(provider_conf),
            signer: self.signer,
        }
    }

    pub fn with_default_rpc_provider(self) -> ChainConfBuilder<T, Defined<ProviderConf>, S> {
        ChainConfBuilder {
            addresses: self.addresses,
            provider_conf: Defined::new(ProviderConf::Rpc(RpcConfig {})),
            signer: self.signer,
        }
    }

    pub fn with_default_graphql_provider(self) -> ChainConfBuilder<T, Defined<ProviderConf>, S> {
        ChainConfBuilder {
            addresses: self.addresses,
            provider_conf: Defined::new(ProviderConf::GraphQl(GRAPHQL_PROVIDER.clone().to_owned())),
            signer: self.signer,
        }
    }
}

impl<T, P> ChainConfBuilder<T, P, Undefined<SignerConf>>
where
    T: MaybeDefined<CoreContractAddresses>,
    P: MaybeDefined<ProviderConf>,
{
    pub fn with_signer(self, signer: SignerConf) -> ChainConfBuilder<T, P, Defined<SignerConf>> {
        ChainConfBuilder {
            addresses: self.addresses,
            provider_conf: self.provider_conf,
            signer: Defined::new(signer),
        }
    }
}

impl<T, S> ChainConfBuilder<T, Defined<ProviderConf>, S>
where
    T: MaybeDefined<CoreContractAddresses>,
    S: MaybeDefined<SignerConf>,
{
    pub async fn build(self) -> TestSuite {
        let connection = build_connection_conf(self.provider_conf.into_inner());
        let signer = if let Some(signer_conf) = self.signer.maybe_inner() {
            Some(signer_conf.build::<DangoSigner>().await.unwrap())
        } else {
            None
        };
        let provider = DangoProvider::from_config(&connection, DANGO_DOMAIN, signer).unwrap();

        // Query chain to retrieve the dango addresses (warp and hyperlane).
        let app_addresses = provider
            .query_app_config::<AppConfig>()
            .await
            .unwrap()
            .addresses;

        // If the addresses are provided, use them; otherwise, query the app config.
        let addresses = if let Some(addresses) = self.addresses.maybe_into_inner() {
            addresses
        } else {
            CoreContractAddresses {
                mailbox: app_addresses.hyperlane.mailbox.convert(),
                interchain_gas_paymaster: app_addresses.hyperlane.fee.convert(),
                validator_announce: app_addresses.hyperlane.va.convert(),
                merkle_tree_hook: app_addresses.hyperlane.merkle.convert(),
            }
        };

        let chain_conf = ChainConf {
            domain: DANGO_DOMAIN,
            signer: self.signer.maybe_into_inner(),
            reorg_period: ReorgPeriod::None,
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
        };

        TestSuite {
            chain_conf,
            dango_provider: provider,
            warp_address: app_addresses.warp,
        }
    }
}

impl<P, S> ChainConfBuilder<Undefined<CoreContractAddresses>, P, S>
where
    P: MaybeDefined<ProviderConf>,
    S: MaybeDefined<SignerConf>,
{
    pub fn with_addresses(
        self,
        addresses: CoreContractAddresses,
    ) -> ChainConfBuilder<Defined<CoreContractAddresses>, P, S> {
        ChainConfBuilder {
            addresses: Defined::new(addresses),
            provider_conf: self.provider_conf,
            signer: self.signer,
        }
    }
}

pub struct TestSuite {
    pub chain_conf: ChainConf,
    pub dango_provider: DangoProvider,
    pub warp_address: Addr,
}
