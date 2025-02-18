use {
    super::scope_child::ScopeChild,
    hyperlane_base::settings::{CheckpointSyncerConf, SignerConf},
    std::{
        collections::BTreeMap,
        path::PathBuf,
        process::{Command, Stdio},
    },
};

#[derive(Default)]
pub struct AgentBuilder<'a> {
    agent: Agent,
    checkpoint_syncer: Option<CheckpointSyncerConf>,
    origin_chain_name: Option<OriginChainName>,
    chain_signers: BTreeMap<&'a str, SignerConf>,
    validator_signer: Option<ValidatorSigner>,
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

    pub fn with_chain_signer(mut self, chain: &'a str, signer: SignerConf) -> Self {
        self.chain_signers.insert(chain, signer);
        self
    }

    pub fn launch(self) -> ScopeChild {
        ScopeChild::new(
            Command::new("cargo")
                .args(&["run", "--bin"])
                .args(self.agent.args())
                .arg("--")
                .args(self.origin_chain_name.args())
                .args(self.checkpoint_syncer.args())
                .args(self.chain_signers.args())
                .args(self.validator_signer.args())
                .current_dir(workspace())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .unwrap(),
        )
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
}

impl Args for Agent {
    fn args(self) -> Vec<String> {
        match self {
            Self::Validator => vec!["validator".to_owned()],
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
