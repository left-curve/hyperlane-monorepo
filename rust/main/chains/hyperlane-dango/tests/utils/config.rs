use {
    dango_types::config::AppConfig,
    ethers_prometheus::middleware::PrometheusMiddlewareConf,
    grug::{Addr, Coin, Defined, MaybeDefined, QueryClientExt, Undefined},
    hyperlane_base::settings::{
        ChainConf, ChainConnectionConf, CoreContractAddresses, IndexSettings, SignerConf,
    },
    hyperlane_core::{HyperlaneDomain, KnownHyperlaneDomain, ReorgPeriod, H256},
    hyperlane_dango::{
        ConnectionConf, DangoConvertor, DangoProvider, DangoSigner, GraphQlConfig, ProviderConf,
        RpcConfig,
    },
    std::{collections::HashMap, sync::LazyLock},
    url::Url,
};

pub const DANGO_DOMAIN: HyperlaneDomain = HyperlaneDomain::Known(KnownHyperlaneDomain::Dango);

pub const DEFAULT_RPC_PORT: u16 = 26657;
pub const DEFAULT_RPC_URL: LazyLock<String> =
    LazyLock::new(|| format!("http://localhost:{DEFAULT_RPC_PORT}"));

const GRAPHQL_PROVIDER: LazyLock<GraphQlConfig> = LazyLock::new(|| GraphQlConfig { url: todo!() });

pub fn build_connection_conf(provider_conf: ProviderConf, chain_id: String) -> ConnectionConf {
    ConnectionConf {
        provider_conf,
        gas_price: Coin::new("uusdc", 0).unwrap(),
        gas_scale: 1.2,
        flat_gas_increase: 100_000,
        search_sleep_duration: 2,
        search_retry_attempts: 5,
        chain_id,
        rpcs: vec![Url::parse(DEFAULT_RPC_URL.as_str()).unwrap()],
        operation_batch: Default::default(),
    }
}

pub struct ChainConfBuilder<T, P, S>
where
    T: MaybeDefined<CoreContractAddresses>,
    P: MaybeDefined<ProviderConf>,
    S: MaybeDefined<SignerConf>,
{
    chain_id: String,
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
    pub fn new(chain_id: String) -> Self {
        Self {
            chain_id,
            addresses: Undefined::new(),
            provider_conf: Undefined::new(),
            signer: Undefined::new(),
        }
    }
}

// ProviderConf
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
            chain_id: self.chain_id,
            addresses: self.addresses,
            provider_conf: Defined::new(provider_conf),
            signer: self.signer,
        }
    }

    pub fn with_default_rpc_provider(self) -> ChainConfBuilder<T, Defined<ProviderConf>, S> {
        ChainConfBuilder {
            chain_id: self.chain_id,
            addresses: self.addresses,
            provider_conf: Defined::new(ProviderConf::Rpc(RpcConfig {
                url: DEFAULT_RPC_URL.to_string(),
            })),
            signer: self.signer,
        }
    }

    pub fn with_default_graphql_provider(self) -> ChainConfBuilder<T, Defined<ProviderConf>, S> {
        ChainConfBuilder {
            chain_id: self.chain_id,
            addresses: self.addresses,
            provider_conf: Defined::new(ProviderConf::GraphQl(GRAPHQL_PROVIDER.clone().to_owned())),
            signer: self.signer,
        }
    }
}

// SignerConf
impl<T, P> ChainConfBuilder<T, P, Undefined<SignerConf>>
where
    T: MaybeDefined<CoreContractAddresses>,
    P: MaybeDefined<ProviderConf>,
{
    pub fn with_signer(self, signer: SignerConf) -> ChainConfBuilder<T, P, Defined<SignerConf>> {
        ChainConfBuilder {
            chain_id: self.chain_id,
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
        let connection =
            build_connection_conf(self.provider_conf.into_inner(), self.chain_id.clone());
        let signer = if let Some(signer_conf) = self.signer.maybe_inner() {
            Some(signer_conf.build::<DangoSigner>().await.unwrap())
        } else {
            None
        };
        let provider = DangoProvider::from_config(&connection, DANGO_DOMAIN, signer).unwrap();

        // Query chain to retrieve the dango addresses.
        let app_addresses = provider
            .query_app_config::<AppConfig>(None)
            .await
            .unwrap()
            .addresses;

        // If the addresses are provided, use them; otherwise, query the app config.
        let addresses = if let Some(addresses) = self.addresses.maybe_into_inner() {
            addresses
        } else {
            CoreContractAddresses {
                mailbox: app_addresses.hyperlane.mailbox.convert(),
                interchain_gas_paymaster: H256([0; 32]), // We don't have IGP.
                validator_announce: app_addresses.hyperlane.va.convert(),
                merkle_tree_hook: app_addresses.hyperlane.mailbox.convert(),
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
            chain_id: self.chain_id,
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
