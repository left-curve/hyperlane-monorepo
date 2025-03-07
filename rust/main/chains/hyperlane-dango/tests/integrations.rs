use {
    dango_types::{
        account_factory::{self, AccountType, Username},
        auth::Key,
        bank,
        constants::DANGO_DENOM,
        warp::Route,
    },
    grug::{
        btree_map, btree_set, Addr, Addressable, Coin, Coins, Denom, EncodedBytes, GasOption,
        HashExt, Json, Message, NonEmpty, NumberConst, ResultExt, Signer, StdResult, Tx, Uint128,
        UnsignedTx,
    },
    hyperlane_base::settings::CheckpointSyncerConf,
    hyperlane_core::H256,
    hyperlane_dango::DangoProviderInterface,
    process_terminal::{tprintln, KeyCode, MessageSettings, ProcessSettings, ScrollSettings},
    startup::{startup, AgentOutput, SetupChain, StartupResponse},
    std::{
        str::FromStr,
        thread::{self, sleep},
        time::Duration,
    },
    utils::{
        agent::{Agent, AgentBuilder},
        chain_helper::ClientExt,
        constants::{DANGO1_DOMAIN, DANGO2_DOMAIN},
        crypto::{derive_pk, ValidatorKey},
        dango_builder::{kill_docker_processes, DangoBuilder},
    },
};

pub mod utils;

mod startup {

    use {
        crate::utils::{
            agent::{workspace, Agent, AgentBuilder},
            chain_helper::ChainHelper,
            constants::{DANGO1_DOMAIN, DANGO2_DOMAIN},
            crypto::ValidatorKey,
            dango_builder::{kill_docker_processes, DangoBuilder},
        },
        dango_types::warp::Route,
        grug::{btree_set, Addr, Denom, HexByteArray, NumberConst, ResultExt, Uint128},
        hyperlane_base::settings::CheckpointSyncerConf,
        process_terminal::{tprintln, KeyCode, MessageSettings, ProcessSettings, ScrollSettings},
        std::{
            collections::BTreeSet,
            fs::{create_dir_all, remove_dir_all},
            process::Child,
            thread::sleep,
            time::Duration,
        },
    };

    pub struct SetupChain {
        pub validators: Vec<AgentOutput>,
        pub threshold: u32,
        pub routes: Vec<Denom>,
    }

    #[derive(Clone)]
    pub enum AgentOutput {
        Ignore,
        Terminal,
    }

    impl AgentOutput {
        pub fn piped(&self) -> bool {
            match self {
                AgentOutput::Ignore => false,
                AgentOutput::Terminal => true,
            }
        }
    }

    pub struct StartupResponse {
        pub dango1_ch: ChainHelper,
        pub dango2_ch: ChainHelper,
    }

    fn launch_validators(
        validators: Vec<AgentOutput>,
        chain_name: &str,
        ch: &ChainHelper,
        inital_metrics_port: u16,
    ) -> Vec<(ValidatorKey, Option<Child>)> {
        validators
            .into_iter()
            .enumerate()
            .map(|(i, output)| {
                let vk = ValidatorKey::new_random();

                let child = AgentBuilder::new(Agent::Validator)
                    .with_origin_chain_name(chain_name)
                    .with_checkpoint_syncer(CheckpointSyncerConf::LocalStorage {
                        path: format!("tmp/validators/cs_{chain_name}_{i}").into(),
                    })
                    .with_validator_signer(vk.key.clone())
                    .with_chain_signer(chain_name, &ch.accounts["user_2"])
                    .with_metrics_port(inital_metrics_port - i as u16)
                    .with_db(&format!("tmp/validators/db_{chain_name}_{i}"))
                    .piped(output.piped())
                    .launch();

                tprintln!(
                    "Validator-{chain_name}-{i} launched with PID {}",
                    child.id()
                );

                let child = if let AgentOutput::Terminal = output {
                    process_terminal::add_process(
                        &format!("Validator-{chain_name}-{i}"),
                        child,
                        ProcessSettings::new_with_scroll(
                            MessageSettings::All,
                            ScrollSettings::enable(KeyCode::Up, KeyCode::Down),
                        ),
                    )
                    .unwrap();
                    None
                } else {
                    Some(child)
                };

                (vk, child)
            })
            .collect::<Vec<_>>()
    }

    pub async fn startup(
        relayer: AgentOutput,
        dango1: SetupChain,
        dango2: SetupChain,
    ) -> anyhow::Result<StartupResponse> {
        let ((mut ch1, _), (mut ch2, _)) = (tokio::try_join!(
            DangoBuilder::new("dango1")
                .with_hyperlane_domain(DANGO1_DOMAIN)
                .start(),
            DangoBuilder::new("dango2")
                .with_hyperlane_domain(DANGO2_DOMAIN)
                .with_rpc_port(36657)
                .start()
        ))?;

        // Create tmp dirs
        create_dir_all(workspace().join("tmp/validators")).unwrap();

        process_terminal::with_exit_callback(|| {
            kill_docker_processes(&["dango1", "dango2"]);
            remove_dir_all(workspace().join("tmp")).unwrap();
        });

        let d1_validators = launch_validators(dango1.validators, "dango1", &ch1, 9190);
        let d2_validators = launch_validators(dango2.validators, "dango2", &ch2, 9290);

        // Run relayer
        let relayer_child = AgentBuilder::new(Agent::Relayer)
            .with_origin_chain_name("dango1")
            .with_relay_chains(btree_set!("dango1", "dango2"))
            .with_chain_signer("dango1", &ch1.accounts["user_2"])
            .with_chain_signer("dango2", &ch2.accounts["user_2"])
            .with_db("tmp/relayer")
            .with_allow_local_checkpoint_syncer(true)
            .piped(relayer.piped())
            .launch();

        if let AgentOutput::Terminal = relayer {
            process_terminal::add_process(
                "Relayer",
                relayer_child,
                ProcessSettings::new_with_scroll(
                    MessageSettings::All,
                    ScrollSettings::enable(KeyCode::Up, KeyCode::Down),
                ),
            )
            .unwrap();
        }

        // Set validator sets
        {
            set_validator_set(
                &mut ch1,
                DANGO2_DOMAIN,
                dango2.threshold,
                d2_validators.iter().map(|v| v.0.address.clone()).collect(),
            )
            .await;

            set_validator_set(
                &mut ch2,
                DANGO1_DOMAIN,
                dango1.threshold,
                d1_validators.iter().map(|v| v.0.address.clone()).collect(),
            )
            .await;
        }

        sleep(Duration::from_secs(2));

        // Set routes
        {
            set_routes(
                &mut ch1,
                ch2.cfg.addresses.warp,
                DANGO2_DOMAIN,
                dango1.routes,
            )
            .await;
            set_routes(
                &mut ch2,
                ch1.cfg.addresses.warp,
                DANGO1_DOMAIN,
                dango2.routes,
            )
            .await;
        }

        Ok(StartupResponse {
            dango1_ch: ch1,
            dango2_ch: ch2,
        })
    }

    async fn set_validator_set(
        ch: &mut ChainHelper,
        domain: u32,
        threshold: u32,
        validator_addresses: BTreeSet<HexByteArray<20>>,
    ) {
        ch.set_hyperlane_validators(domain, threshold, validator_addresses)
            .await
            .unwrap()
            .should_succeed();
    }

    async fn set_routes(
        local_ch: &mut ChainHelper,
        destination_warp_addr: Addr,
        destination_domain: u32,
        routes: Vec<Denom>,
    ) {
        for route in routes {
            local_ch
                .set_route(
                    route,
                    destination_domain,
                    Route {
                        address: destination_warp_addr.into(),
                        fee: Uint128::ZERO,
                    },
                )
                .await
                .unwrap()
                .should_succeed();
        }
    }
}

#[tokio::test]
async fn relayer_single_validator() {
    let ((mut ch1, _), (mut ch2, _)) = try_start_test!(tokio::try_join!(
        DangoBuilder::new("dango1")
            .with_hyperlane_domain(DANGO1_DOMAIN)
            .start(),
        DangoBuilder::new("dango2")
            .with_hyperlane_domain(DANGO2_DOMAIN)
            .with_rpc_port(36657)
            .start()
    ));

    process_terminal::with_exit_callback(|| kill_docker_processes(&["dango1", "dango2"]));

    // run Relayer
    {
        let agent = AgentBuilder::new(Agent::Relayer)
            .with_origin_chain_name("dango1")
            .with_relay_chains(btree_set!("dango1", "dango2"))
            .with_chain_signer("dango2", &ch2.accounts["user_2"])
            .with_allow_local_checkpoint_syncer(true)
            .launch();

        process_terminal::add_process(
            "Relayer",
            agent,
            ProcessSettings::new_with_scroll(
                MessageSettings::All,
                ScrollSettings::enable(KeyCode::Up, KeyCode::Down),
            ),
        )
        .unwrap();
    }

    let validator_key = ValidatorKey::new_random();

    // run Validator for dango1
    {
        let validator = AgentBuilder::new(Agent::Validator)
            .with_origin_chain_name("dango1")
            .with_checkpoint_syncer(CheckpointSyncerConf::LocalStorage {
                path: "dango_1".into(),
            })
            .with_validator_signer(validator_key.key.clone())
            .with_chain_signer("dango1", &ch1.accounts["user_2"])
            .with_metrics_port(9089)
            .launch();

        process_terminal::add_process(
            "Validator",
            validator,
            ProcessSettings::new_with_scroll(
                MessageSettings::All,
                ScrollSettings::enable(KeyCode::Up, KeyCode::Down),
            ),
        )
        .unwrap();
    }

    // Set route on dango1
    {
        tprintln!("Setting route on dango1...");
        ch1.set_route(
            DANGO_DENOM.clone(),
            DANGO2_DOMAIN,
            Route {
                address: ch2.cfg.addresses.warp.into(),
                fee: Uint128::ZERO,
            },
        )
        .await
        .unwrap()
        .should_succeed();
        tprintln!("Route set on dango1");
    }

    let dango_2_denom = Denom::from_str("hyp/d1/dango").unwrap();

    // Set route on dango2
    {
        tprintln!("Setting route on dango2...");
        ch2.set_route(
            dango_2_denom.clone(),
            DANGO1_DOMAIN,
            Route {
                address: ch1.cfg.addresses.warp.into(),
                fee: Uint128::ZERO,
            },
        )
        .await
        .unwrap()
        .should_succeed();
        tprintln!("Route set on dango2");
    }

    thread::sleep(Duration::from_secs(2));

    // Set validator set on dango1
    {
        tprintln!("Setting validator set on dango1...");
        ch1.set_hyperlane_validators(DANGO2_DOMAIN, 1, btree_set!(validator_key.address.clone()))
            .await
            .unwrap()
            .should_succeed();
        tprintln!("Validator set set on dango1");
    }

    // Set validator set on dango2
    {
        tprintln!("Setting validator set on dango2...");
        ch2.set_hyperlane_validators(DANGO1_DOMAIN, 1, btree_set!(validator_key.address.clone()))
            .await
            .unwrap()
            .should_succeed();
        tprintln!("Validator set set on dango2");
    }

    // Wait until validator start
    {
        let msg = process_terminal::block_search_message("Validator", "Waiting for").unwrap();
        tprintln!("msg: {}", msg);
    }

    // Transfer from dango1 to dango2
    {
        tprintln!("Transferring from dango1 to dango2...");
        ch1.send_remote(
            "user_1",
            Coin::new(DANGO_DENOM.clone(), 100).unwrap(),
            DANGO2_DOMAIN,
            ch2.accounts["user_1"].address,
        )
        .await
        .unwrap()
        .should_succeed();
        tprintln!("Transferred from dango1 to dango2");
    }

    loop {
        let balances = ch2
            .client
            .query_balances(ch2.accounts["user_1"].address, None, None, None)
            .await
            .unwrap();

        tprintln!("balances: {:?}", balances);

        std::thread::sleep(std::time::Duration::from_secs(1));
    }
}

#[tokio::test]
async fn relayer_with_triple_validator() {
    let dango2_denom = Denom::from_str("hyp/dango1/dango").unwrap();

    let StartupResponse {
        mut dango1_ch,
        dango2_ch,
    } = try_start_test!(
        startup(
            AgentOutput::Terminal,
            SetupChain {
                validators: vec![AgentOutput::Ignore; 3],
                threshold: 2,
                routes: vec![DANGO_DENOM.clone()],
            },
            SetupChain {
                validators: vec![AgentOutput::Ignore],
                threshold: 1,
                routes: vec![dango2_denom.clone()],
            },
        )
        .await
    );

    sleep(Duration::from_secs(5));

    // Transfer from dango1 to dango2
    {
        tprintln!("Transferring from dango1 to dango2...");
        dango1_ch
            .send_remote(
                "user_1",
                Coin::new(DANGO_DENOM.clone(), 100).unwrap(),
                DANGO2_DOMAIN,
                dango2_ch.accounts["user_1"].address,
            )
            .await
            .unwrap()
            .should_succeed();
        tprintln!("Transferred from dango1 to dango2!");
    }

    loop {
        let balances = dango2_ch
            .client
            .query_balances(dango2_ch.accounts["user_1"].address, None, None, None)
            .await
            .unwrap();

        tprintln!("balances: {:?}", balances);

        std::thread::sleep(std::time::Duration::from_secs(1));
    }
}

#[tokio::test]
async fn onboarding() {
    let dango2_denom = Denom::from_str("hyp/dango1/dng").unwrap();

    let StartupResponse {
        mut dango1_ch,
        dango2_ch,
    } = try_start_test!(
        startup(
            AgentOutput::Terminal,
            SetupChain {
                validators: vec![AgentOutput::Ignore],
                threshold: 1,
                routes: vec![DANGO_DENOM.clone()],
            },
            SetupChain {
                validators: vec![AgentOutput::Ignore],
                threshold: 1,
                routes: vec![dango2_denom.clone()],
            },
        )
        .await
    );

    // Compute the address of the non existing account
    let sk = H256::random();
    let pk = derive_pk(&sk);
    let key_hash = pk.hash256();
    let key = Key::Secp256k1(EncodedBytes::from_inner(pk));

    let salt = dango_types::account_factory::NewUserSalt {
        secret: 10,
        key: key.clone(),
        key_hash,
    }
    .into_bytes();

    // get code_hash for spot account
    let code_hash = dango1_ch
        .client
        .query_wasm_smart(
            dango1_ch.cfg.addresses.account_factory,
            account_factory::QueryCodeHashRequest {
                account_type: AccountType::Spot,
            },
            None,
        )
        .await
        .unwrap();

    let user_addr = Addr::derive(dango1_ch.cfg.addresses.account_factory, code_hash, &salt);

    // Transfer from dango1 to dango2 into a non existing account
    {
        tprintln!("Transferring from dango1 to dango2...");
        dango1_ch
            .send_remote(
                "user_1",
                Coin::new(DANGO_DENOM.clone(), 100).unwrap(),
                DANGO2_DOMAIN,
                user_addr,
            )
            .await
            .unwrap()
            .should_succeed();
        tprintln!("Transferred from dango1 to dango2!");
    }

    tprintln!("Waiting for orphaned transfer...");

    // Wait until there is an orphaned transfer
    let bank = dango2_ch.client.query_config(None).await.unwrap().bank;
    loop {
        if let Ok(coins) = dango2_ch
            .client
            .query_wasm_smart(
                bank,
                bank::QueryOrphanedTransferRequest {
                    sender: dango2_ch.cfg.addresses.warp,
                    recipient: user_addr,
                },
                None,
            )
            .await
        {
            if !coins.is_empty() {
                tprintln!("Orphaned transfer found!");
                break;
            }
        }
        sleep(Duration::from_secs(1));
    }

    // Finalize user onboarding
    {
        let mut signer = FactorySigner {
            address: dango2_ch.cfg.addresses.account_factory,
        };

        dango2_ch
            .client
            .broadcast_and_find(
                &mut signer,
                Message::execute(
                    dango2_ch.cfg.addresses.account_factory,
                    &account_factory::ExecuteMsg::RegisterUser {
                        username: Username::from_str("user_3").unwrap(),
                        secret: 10,
                        key,
                        key_hash,
                    },
                    Coins::default(),
                )
                .unwrap(),
                GasOption::Predefined {
                    gas_limit: 10_000_000,
                },
            )
            .await
            .unwrap()
            .should_succeed();

        tprintln!("Broadcasted register user!");

        let balance = dango2_ch
            .client
            .query_balances(user_addr, None, None, None)
            .await
            .unwrap();

        assert_eq!(
            balance,
            Coins::new_unchecked(btree_map! {
                dango2_denom => Uint128::from(100u128)
            })
        );
    }

    process_terminal::end_terminal();
}

struct FactorySigner {
    address: Addr,
}

impl Addressable for FactorySigner {
    fn address(&self) -> Addr {
        self.address
    }
}

impl Signer for FactorySigner {
    fn unsigned_transaction(
        &self,
        msgs: NonEmpty<Vec<Message>>,
        _chain_id: &str,
    ) -> StdResult<UnsignedTx> {
        Ok(UnsignedTx {
            sender: self.address(),
            msgs,
            data: Json::null(),
        })
    }

    fn sign_transaction(
        &mut self,
        msgs: NonEmpty<Vec<Message>>,
        _chain_id: &str,
        gas_limit: u64,
    ) -> StdResult<Tx> {
        Ok(Tx {
            sender: self.address,
            gas_limit,
            msgs,
            data: Json::null(),
            credential: Json::null(),
        })
    }
}
