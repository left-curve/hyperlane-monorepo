use {
    super::user::IntoSignerConf,
    hyperlane_base::settings::{CheckpointSyncerConf, SignerConf},
    std::{
        collections::{BTreeMap, BTreeSet},
        path::PathBuf,
        process::{Child, Command, Stdio},
    },
};

#[derive(Default)]
pub struct AgentBuilder<'a> {
    agent: Agent,
    checkpoint_syncer: Option<CheckpointSyncerConf>,
    origin_chain_name: Option<OriginChainName>,
    allow_local_checkpoint_syncer: Option<AllowLocalCheckpointSyncer>,
    chain_signers: BTreeMap<&'a str, SignerConf>,
    validator_signer: Option<ValidatorSigner>,
    relay_chains: Option<RelayChains<'a>>,
    metrics_port: Option<MetricsPort>,
}

impl<'a> AgentBuilder<'a> {
    pub fn new(agent: Agent) -> Self {
        Self {
            agent,
            ..Default::default()
        }
    }

    pub fn with_origin_chain_name(mut self, origin_chain_name: &str) -> Self {
        self.origin_chain_name = Some(OriginChainName(origin_chain_name.to_string()));
        self
    }

    pub fn with_checkpoint_syncer(mut self, checkpoint_syncer: CheckpointSyncerConf) -> Self {
        self.checkpoint_syncer = Some(checkpoint_syncer);
        self
    }

    pub fn with_validator_signer(mut self, signer: SignerConf) -> Self {
        self.validator_signer = Some(ValidatorSigner(signer));
        self
    }

    pub fn with_allow_local_checkpoint_syncer(
        mut self,
        allow_local_checkpoint_syncer: bool,
    ) -> Self {
        self.allow_local_checkpoint_syncer =
            Some(AllowLocalCheckpointSyncer(allow_local_checkpoint_syncer));
        self
    }

    pub fn with_chain_signer<S>(mut self, chain: &'a str, signer: S) -> Self
    where
        S: IntoSignerConf,
    {
        self.chain_signers.insert(chain, signer.as_signer_conf());
        self
    }

    pub fn with_relay_chains(mut self, relay_chains: BTreeSet<&'a str>) -> Self {
        self.relay_chains = Some(RelayChains(relay_chains));
        self
    }

    pub fn with_metrics_port(mut self, metrics_port: u16) -> Self {
        self.metrics_port = Some(MetricsPort(metrics_port));
        self
    }

    pub fn launch(self) -> Child {
        Command::new("cargo")
            .args(&["run", "--bin"])
            .args(self.agent.args())
            .arg("--")
            .args(self.origin_chain_name.args())
            .args(self.checkpoint_syncer.args())
            .args(self.chain_signers.args())
            .args(self.validator_signer.args())
            .args(self.relay_chains.args())
            .args(self.allow_local_checkpoint_syncer.args())
            .args(self.metrics_port.args())
            .current_dir(workspace())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap()
    }
}

fn workspace() -> PathBuf {
    let target_subpath = "hyperlane-monorepo/rust/main";

    let current_dir = std::env::current_dir()
        .unwrap()
        .to_string_lossy()
        .into_owned();

    let index = current_dir.find(target_subpath).unwrap();
    let base_path = &current_dir[..index + target_subpath.len()];
    PathBuf::from(base_path)
}

pub trait Args {
    fn args(self) -> Vec<String>;
}

impl<T> Args for Option<T>
where
    T: Args,
{
    fn args(self) -> Vec<String> {
        match self {
            Some(inner) => inner.args(),
            None => vec![],
        }
    }
}

impl Args for BTreeMap<&str, SignerConf> {
    fn args(self) -> Vec<String> {
        self.into_iter()
            .flat_map(|(chain, signer)| {
                ChainSigner {
                    chain: chain.to_string(),
                    signer,
                }
                .args()
            })
            .collect()
    }
}

#[derive(Default)]
pub enum Agent {
    #[default]
    Validator,
    Relayer,
}

impl Args for Agent {
    fn args(self) -> Vec<String> {
        match self {
            Self::Validator => vec!["validator".to_owned()],
            Self::Relayer => vec!["relayer".to_owned()],
        }
    }
}

struct OriginChainName(String);

impl Args for OriginChainName {
    fn args(self) -> Vec<String> {
        vec!["--origin-chain-name".to_string(), self.0]
    }
}

impl Args for CheckpointSyncerConf {
    fn args(self) -> Vec<String> {
        match self {
            Self::LocalStorage { path } => {
                vec![
                    "--checkpointSyncer.type".to_string(),
                    "localStorage".to_string(),
                    "--checkpointSyncer.path".to_string(),
                    path.to_string_lossy().to_string(),
                ]
            }
            _ => unimplemented!(),
        }
    }
}

pub struct ValidatorSigner(SignerConf);

impl Args for ValidatorSigner {
    fn args(self) -> Vec<String> {
        with_signer_config("validator", self.0)
    }
}

fn with_signer_config(prepath: &str, signer_conf: SignerConf) -> Vec<String> {
    match signer_conf {
        SignerConf::HexKey { key } => vec![
            format!("--{prepath}.type"),
            "hexKey".to_string(),
            format!("--{prepath}.key"),
            format!("{:?}", key),
        ],

        SignerConf::Dango {
            username,
            key,
            address,
        } => vec![
            format!("--{prepath}.type"),
            "dango".to_string(),
            format!("--{prepath}.username"),
            username.to_string(),
            format!("--{prepath}.key"),
            key.to_string(),
            format!("--{prepath}.address"),
            address.to_string(),
        ],
        _ => unimplemented!(),
    }
}

pub struct ChainSigner {
    chain: String,
    signer: SignerConf,
}

impl Args for ChainSigner {
    fn args(self) -> Vec<String> {
        with_signer_config(&format!("chains.{}.signer", self.chain), self.signer)
    }
}

pub struct RelayChains<'a>(BTreeSet<&'a str>);

impl Args for RelayChains<'_> {
    fn args(self) -> Vec<String> {
        vec![
            "--relayChains".to_string(),
            self.0.into_iter().collect::<Vec<_>>().join(","),
        ]
    }
}

pub struct AllowLocalCheckpointSyncer(bool);

impl Args for AllowLocalCheckpointSyncer {
    fn args(self) -> Vec<String> {
        vec![
            "--allowLocalCheckpointSyncer".to_string(),
            self.0.to_string(),
        ]
    }
}

pub struct MetricsPort(u16);

impl Args for MetricsPort {
    fn args(self) -> Vec<String> {
        vec!["--metrics-port".to_string(), self.0.to_string()]
    }
}
