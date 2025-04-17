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

pub struct DangoBuilder<HD = Undefined<u32>, RPC = Undefined<u16>>
where
    HD: MaybeDefined<u32>,
    RPC: MaybeDefined<u16>,
{
    container_name: String,
    hyperlane_domain: HD,
    port: RPC,
}

impl DangoBuilder {
    pub fn new(container_name: &str) -> Self {
        Self {
            container_name: container_name.to_string(),
            hyperlane_domain: Undefined::default(),
            port: Undefined::default(),
        }
    }
}

impl<RPC> DangoBuilder<Undefined<u32>, RPC>
where
    RPC: MaybeDefined<u16>,
{
    pub fn with_hyperlane_domain(self, hyperlane_domain: u32) -> DangoBuilder<Defined<u32>, RPC> {
        DangoBuilder {
            container_name: self.container_name,
            hyperlane_domain: Defined::new(hyperlane_domain),
            port: self.port,
        }
    }
}

impl<HD> DangoBuilder<HD, Undefined<u16>>
where
    HD: MaybeDefined<u32>,
{
    pub fn with_rpc_port(self, port: u16) -> DangoBuilder<HD, Defined<u16>> {
        DangoBuilder {
            container_name: self.container_name,
            hyperlane_domain: self.hyperlane_domain,
            port: Defined::new(port),
        }
    }
}

impl<HD, RPC> DangoBuilder<HD, RPC>
where
    HD: MaybeDefined<u32>,
    RPC: MaybeDefined<u16>,
{
    pub async fn start(self) -> anyhow::Result<(ChainHelper, Child)> {
        let port = self.port.maybe_into_inner().unwrap_or(26657);

        let client = TendermintRpcClient::new(format!("http://localhost:{port}").as_str()).unwrap();

        let child = start_dango_docker(
            self.container_name.as_str(),
            port,
            self.hyperlane_domain.maybe_into_inner().unwrap_or(88888888),
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

        Ok((ChainHelper::new(client, accounts, chain_id).await?, child))
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
