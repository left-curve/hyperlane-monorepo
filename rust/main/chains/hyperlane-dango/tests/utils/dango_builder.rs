use {
    super::constants::COIN_TYPE,
    crate::utils::chain_helper::ChainHelper,
    anyhow::{bail, ensure, Ok},
    dango_client::SingleSigner,
    grug::{
        Binary, BlockClient, BorshDeExt, Defined, JsonDeExt, MaybeDefined, QueryClient,
        TendermintRpcClient, Undefined,
    },
    process_terminal::tprintln,
    serde::de::DeserializeOwned,
    std::{
        collections::BTreeMap,
        process::{Child, Command, Stdio},
        str::FromStr,
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

pub struct DangoBuilder<HD = Undefined<u32>, RP = Undefined<u16>, GP = Undefined<u16>>
where
    HD: MaybeDefined<u32>,
    RP: MaybeDefined<u16>,
    GP: MaybeDefined<u16>,
{
    container_name: String,
    hyperlane_domain: HD,
    port_rpc: RP,
    port_graphql: GP,
}

impl DangoBuilder {
    pub fn new(container_name: &str) -> Self {
        Self {
            container_name: container_name.to_string(),
            hyperlane_domain: Undefined::default(),
            port_rpc: Undefined::default(),
            port_graphql: Undefined::default(),
        }
    }
}

impl<RP, GP> DangoBuilder<Undefined<u32>, RP, GP>
where
    RP: MaybeDefined<u16>,
    GP: MaybeDefined<u16>,
{
    pub fn with_hyperlane_domain(
        self,
        hyperlane_domain: u32,
    ) -> DangoBuilder<Defined<u32>, RP, GP> {
        DangoBuilder {
            container_name: self.container_name,
            hyperlane_domain: Defined::new(hyperlane_domain),
            port_rpc: self.port_rpc,
            port_graphql: self.port_graphql,
        }
    }
}

impl<HD, GP> DangoBuilder<HD, Undefined<u16>, GP>
where
    HD: MaybeDefined<u32>,
    GP: MaybeDefined<u16>,
{
    pub fn with_rpc_port(self, port: u16) -> DangoBuilder<HD, Defined<u16>, GP> {
        DangoBuilder {
            container_name: self.container_name,
            hyperlane_domain: self.hyperlane_domain,
            port_rpc: Defined::new(port),
            port_graphql: self.port_graphql,
        }
    }
}

impl<HD, RP> DangoBuilder<HD, RP, Undefined<u16>>
where
    HD: MaybeDefined<u32>,
    RP: MaybeDefined<u16>,
{
    pub fn with_graphql_port(self, port: u16) -> DangoBuilder<HD, RP, Defined<u16>> {
        DangoBuilder {
            container_name: self.container_name,
            hyperlane_domain: self.hyperlane_domain,
            port_rpc: self.port_rpc,
            port_graphql: Defined::new(port),
        }
    }
}

impl<HD, RP, GP> DangoBuilder<HD, RP, GP>
where
    HD: MaybeDefined<u32>,
    RP: MaybeDefined<u16>,
    GP: MaybeDefined<u16>,
{
    pub async fn start(self) -> anyhow::Result<(ChainHelper, Child)> {
        let rpc_port = self.port_rpc.maybe_into_inner().unwrap_or(26657);

        let client =
            TendermintRpcClient::new(format!("http://localhost:{rpc_port}").as_str()).unwrap();

        let hyperlane_domain = self.hyperlane_domain.maybe_into_inner().unwrap_or(88888888);

        let child = start_dango_docker(
            self.container_name.as_str(),
            rpc_port,
            hyperlane_domain,
            &client,
        )
        .await?;

        let chain_id: String = client
            .query_store(Binary::from_str("Y2hhaW5faWQ=").unwrap(), None, false)
            .await
            .map_err(|e| anyhow::Error::from(e))?
            .0
            .unwrap()
            .to_vec()
            .deserialize_borsh()
            .unwrap();

        println!("Chain ID: {}", chain_id);

        let genesis: dangod_types::Genesis =
            read_docker_file(&self.container_name, "/root/.dangod/genesis.json").unwrap();

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
            .await.unwrap();

            accounts.insert(username, signer);
        }

        Ok((
            ChainHelper::new(client, accounts, chain_id, hyperlane_domain).await?,
            child,
        ))
    }
}

async fn await_until_chain_start(client: &TendermintRpcClient) {
    tprintln!("waiting for chain to start...");

    loop {
        if client.query_block(None).await.is_ok() {
            tprintln!("chain started!");
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
}

fn is_docker_running() -> bool {
    Command::new("docker")
        .arg("info")
        .output()
        .map_or(false, |output| output.status.success())
}

async fn start_dango_docker(
    chain_name: &str,
    port: u16,
    hyperlane_domain: u32,
    client: &TendermintRpcClient,
) -> Result<Child, anyhow::Error> {
    ensure!(
        is_docker_running(),
        "docker is not running, please start it"
    );

    let child = Command::new("docker")
        .args(&[
            "run",
            "--rm",
            "--name",
            chain_name,
            "-p",
            &format!("{port}:26657"),
            "dango",
            "--hyperlane-domain",
            &format!("{hyperlane_domain}"),
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| anyhow::Error::from(e))?;

    await_until_chain_start(client).await;

    Ok(child)
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

pub fn kill_docker_processes(container_names: &[&str]) {
    println!("Exiting processes");
    for name in container_names {
        Command::new("docker")
            .args(&["kill", name])
            .output()
            .unwrap();
    }
}
