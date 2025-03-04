use {
    super::constants::{CHAIN_ID, COIN_TYPE},
    anyhow::{bail, ensure},
    dango_client::SingleSigner,
    dango_types::auth::Nonce,
    grug::{Client, Defined, JsonDeExt, MaybeDefined, SigningClient, Undefined},
    process_terminal::tprintln,
    serde::de::DeserializeOwned,
    std::{
        collections::BTreeMap,
        ops::{Deref, DerefMut, Index, IndexMut},
        process::{Child, Command, Stdio},
    },
};

#[macro_export]
macro_rules! try_start_test {
    ($fn: expr) => {
        match $fn {
            Ok(ok) => ok,
            Err(err) => {
                println!("Test skipped: {}", err);
                return;
            }
        }
    };
}

pub async fn await_until_chain_start(client: &Client) {
    tprintln!("waiting for chain to start...");

    loop {
        if client.query_block(None).await.is_ok() {
            tprintln!("chain started!");
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
}

pub struct DangodBuilder<HD = Undefined<u32>, RPC = Undefined<u16>>
where
    HD: MaybeDefined<u32>,
    RPC: MaybeDefined<u16>,
{
    container_name: String,
    hyperlane_domain: HD,
    port: RPC,
}

impl DangodBuilder {
    pub fn new(container_name: &str) -> Self {
        Self {
            container_name: container_name.to_string(),
            hyperlane_domain: Undefined::default(),
            port: Undefined::default(),
        }
    }
}

impl<HD, RPC> DangodBuilder<HD, RPC>
where
    HD: MaybeDefined<u32>,
    RPC: MaybeDefined<u16>,
{
    pub async fn start(self) -> anyhow::Result<DangodEnv> {
        let port = self.port.maybe_into_inner().unwrap_or(26657);

        let client =
            SigningClient::connect(CHAIN_ID, format!("http://localhost:{port}").as_str()).unwrap();

        ensure!(
            is_docker_running(),
            "docker is not running, please start it"
        );

        let child = start_dango_docker(
            self.container_name.as_str(),
            port,
            self.hyperlane_domain.maybe_into_inner().unwrap_or(88888888),
        );

        await_until_chain_start(&client).await;

        let genesis: dangod_types::Genesis =
            read_docker_file(&self.container_name, "/root/.dangod/genesis.json")?;

        let mut accounts = BTreeMap::new();

        for (username, account) in &genesis.accounts {
            let username = username.to_string();
            let signer = SingleSigner::from_mnemonic(
                &username,
                account.address.unwrap(),
                &account.mnemonic,
                COIN_TYPE,
            )
            .unwrap()
            .query_nonce(&client)
            .await?;

            accounts.insert(username, signer);
        }

        Ok(DangodEnv {
            client,
            child,
            accounts: Accounts(accounts),
        })
    }
}

impl<RPC> DangodBuilder<Undefined<u32>, RPC>
where
    RPC: MaybeDefined<u16>,
{
    pub fn with_hyperlane_domain(self, hyperlane_domain: u32) -> DangodBuilder<Defined<u32>, RPC> {
        DangodBuilder {
            container_name: self.container_name,
            hyperlane_domain: Defined::new(hyperlane_domain),
            port: self.port,
        }
    }
}

impl<HD> DangodBuilder<HD, Undefined<u16>>
where
    HD: MaybeDefined<u32>,
{
    pub fn with_rpc_port(self, port: u16) -> DangodBuilder<HD, Defined<u16>> {
        DangodBuilder {
            container_name: self.container_name,
            hyperlane_domain: self.hyperlane_domain,
            port: Defined::new(port),
        }
    }
}

pub struct DangodEnv {
    pub child: Child,
    pub accounts: Accounts,
    pub client: SigningClient,
}

pub struct Accounts(BTreeMap<String, SingleSigner<Defined<Nonce>>>);

impl Deref for Accounts {
    type Target = BTreeMap<String, SingleSigner<Defined<Nonce>>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Accounts {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<S> Index<S> for Accounts
where
    S: AsRef<str>,
{
    type Output = SingleSigner<Defined<Nonce>>;

    fn index(&self, index: S) -> &Self::Output {
        self.get(index.as_ref()).expect("account not found")
    }
}

impl<S> IndexMut<S> for Accounts
where
    S: AsRef<str>,
{
    fn index_mut(&mut self, index: S) -> &mut Self::Output {
        self.get_mut(index.as_ref()).expect("account not found")
    }
}

fn is_docker_running() -> bool {
    Command::new("docker").arg("info").output().is_ok()
}

fn start_dango_docker(chain_name: &str, port: u16, hyperlane_domain: u32) -> Child {
    Command::new("docker")
        .args(&[
            "run",
            "--rm",
            "--name",
            chain_name,
            "-p",
            &format!("{port}:26657"),
            "dango",
            &format!("--hyperlane_domain {hyperlane_domain}"),
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap()
}

fn read_docker_file<R>(container_name: &str, file_path: &str) -> anyhow::Result<R>
where
    R: DeserializeOwned,
{
    let output = Command::new("docker")
        .args(["exec", container_name, "cat", file_path])
        .output()?;

    if output.status.success() {
        let str = std::str::from_utf8(&output.stdout)?;

        Ok(str.deserialize_json()?)
    } else {
        bail!(
            "Failed to read file: {}",
            std::str::from_utf8(&output.stderr)?
        )
    }
}
