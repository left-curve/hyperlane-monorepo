use {
    dango_types::{constants::DANGO_DENOM, warp::Route},
    grug::{btree_set, Coin, Denom, NumberConst, Uint128},
    hyperlane_base::settings::{CheckpointSyncerConf, SignerConf},
    process_terminal::{tprintln, KeyCode, MessageSettings, ProcessSettings, ScrollSettings},
    std::{str::FromStr, thread, time::Duration},
    tendermint::abci::Code,
    utils::{
        agent::{Agent, AgentBuilder},
        constants::{DANGO1_DOMAIN, DANGO2_DOMAIN, VALIDATOR_ADDRESS, VALIDATOR_KEY},
        dango_builder::{kill_docker_processes, DangoBuilder},
    },
};

pub mod utils;

#[tokio::test]
async fn relayer() {
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

    // run Validator for dango1
    {
        let validator = AgentBuilder::new(Agent::Validator)
            .with_origin_chain_name("dango1")
            .with_checkpoint_syncer(CheckpointSyncerConf::LocalStorage {
                path: "dango_1".into(),
            })
            .with_validator_signer(SignerConf::HexKey {
                key: VALIDATOR_KEY.clone(),
            })
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
        .unwrap();
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
        .unwrap();
        tprintln!("Route set on dango2");
    }

    thread::sleep(Duration::from_secs(2));

    // Set validator set on dango1
    {
        tprintln!("Setting validator set on dango1...");
        ch1.set_hyperlane_validators(DANGO2_DOMAIN, 1, btree_set!(VALIDATOR_ADDRESS.clone()))
            .await
            .unwrap();
        tprintln!("Validator set set on dango1");
    }

    // Set validator set on dango2
    {
        tprintln!("Setting validator set on dango2...");
        let res = ch2
            .set_hyperlane_validators(DANGO1_DOMAIN, 1, btree_set!(VALIDATOR_ADDRESS.clone()))
            .await
            .unwrap();
        tprintln!("Validator set set on dango2");
        assert_eq!(res.code, Code::Ok, "Tx failed! {:?}", res);
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
        .unwrap();
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
