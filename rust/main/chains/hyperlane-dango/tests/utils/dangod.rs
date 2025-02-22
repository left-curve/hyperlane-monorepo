use {
    super::constants::{CHAIN_ID, COIN_TYPE, LOCALHOST},
    dango_client::SingleSigner,
    dango_types::auth::Nonce,
    dangod_types::{home_dir, PathBuffExt, Writer},
    grug::{Client, Defined, MaybeDefined, Message, SigningClient, Undefined},
    process_terminal::tprintln,
    std::{
        collections::BTreeMap,
        ops::{Deref, DerefMut, Index, IndexMut},
        process::{Child, Command, Stdio},
    },
};

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

fn is_installed(name: &str) -> anyhow::Result<()> {
    let output = Command::new("which").arg(name).output()?;
    if !output.status.success() {
        anyhow::bail!("{} is not installed", name);
    } else {
        Ok(())
    }
}

#[macro_export]
macro_rules! try_start_test {
    ($fn: expr) => {
        match $fn {
            Ok(_) => {}
            Err(e) => {
                println!("Test skipped: {}", e);
                return;
            }
        }
    };
}

type GenesisClosure = Box<dyn FnOnce(&mut dangod_types::Genesis) + Send>;
type MessagesClosure = Box<dyn FnOnce(&dangod_types::Genesis) -> Vec<Message> + Send>;

pub struct DangodBuilder<G = Undefined<GenesisClosure>, M = Undefined<MessagesClosure>>
where
    G: MaybeDefined<GenesisClosure>,
    M: MaybeDefined<MessagesClosure>,
{
    genesis_closure: G,
    genesis_msgs_closure: M,
}

impl DangodBuilder {
    pub fn new() -> Self {
        Self {
            genesis_closure: Undefined::default(),
            genesis_msgs_closure: Undefined::default(),
        }
    }
}

impl<M> DangodBuilder<Undefined<GenesisClosure>, M>
where
    M: MaybeDefined<MessagesClosure>,
{
    pub fn with_genesis_closure<C: FnOnce(&mut dangod_types::Genesis) + Send + 'static>(
        self,
        closure: C,
    ) -> DangodBuilder<Defined<GenesisClosure>, M> {
        DangodBuilder {
            genesis_closure: Defined::new(Box::new(closure)),
            genesis_msgs_closure: self.genesis_msgs_closure,
        }
    }
}

impl<G> DangodBuilder<G, Undefined<MessagesClosure>>
where
    G: MaybeDefined<GenesisClosure>,
{
    pub fn with_extra_messages<
        C: FnOnce(&dangod_types::Genesis) -> Vec<Message> + Send + 'static,
    >(
        self,
        closure: C,
    ) -> DangodBuilder<G, Defined<MessagesClosure>> {
        DangodBuilder {
            genesis_closure: self.genesis_closure,
            genesis_msgs_closure: Defined::new(Box::new(closure)),
        }
    }
}

impl<G, M> DangodBuilder<G, M>
where
    G: MaybeDefined<GenesisClosure>,
    M: MaybeDefined<MessagesClosure>,
{
    pub async fn start(self) -> anyhow::Result<DangodEnv> {
        let client = SigningClient::connect(CHAIN_ID, LOCALHOST).unwrap();

        is_installed("dangod")?;
        is_installed("cometbft")?;
        is_installed("dango")?;

        // Reset dango and cometbft
        Command::new("dangod").arg("reset").status()?;
        Command::new("dangod").arg("generate-static").status()?;

        let path_dangod_config = home_dir()?.join(".dangod/genesis.json");

        if let Some(closure) = self.genesis_closure.maybe_into_inner() {
            let mut genesis: dangod_types::Genesis = path_dangod_config.read()?;
            closure(&mut genesis);
            genesis.write_pretty_json(&path_dangod_config)?;
        }

        Command::new("dangod").arg("build").status()?;

        if let Some(closure) = self.genesis_msgs_closure.maybe_into_inner() {
            let mut genesis: dangod_types::Genesis = path_dangod_config.read()?;
            let msgs = closure(&genesis);
            genesis.extra_msgs = msgs;
            genesis.write_pretty_json(&path_dangod_config)?;

            Command::new("dangod").arg("build-msgs").status()?;
        }

        let genesis: dangod_types::Genesis = path_dangod_config.read()?;

        let child = Command::new("dangod")
            .arg("start")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        await_until_chain_start(&client).await;

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
