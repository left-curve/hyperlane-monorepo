use {
    dango_types::{constants::DANGO_DENOM, warp::Route},
    grug::{Addr, Coin, NumberConst, ResultExt, Uint128},
    hyperlane_base::settings::CheckpointSyncerConf,
    process_terminal::{tprintln, KeyCode, MessageSettings, ProcessSettings, ScrollSettings},
    utils::{
        agent::{Agent, AgentBuilder},
        crypto::ValidatorKey,
        dango_builder::{kill_docker_processes, DangoBuilder},
    },
};

pub mod utils;

#[tokio::test]
async fn run_validator() {
    let (mut ch, child) = try_start_test!(
        DangoBuilder::new("dango")
            .with_hyperlane_domain(88888887)
            .start()
            .await
    );

    process_terminal::add_process(
        "Dango",
        child,
        ProcessSettings::new(MessageSettings::Output),
    )
    .unwrap();

    process_terminal::with_exit_callback(|| kill_docker_processes(&["dango"]));

    let validator = AgentBuilder::new(Agent::Validator)
        .with_origin_chain_name("dango1")
        .with_checkpoint_syncer(CheckpointSyncerConf::LocalStorage {
            path: "dango_1".into(),
        })
        .with_validator_signer(ValidatorKey::new_random().key)
        .with_chain_signer("dango1", &ch.accounts["user_2"])
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

    // Set route
    ch.set_route(
        DANGO_DENOM.clone(),
        10,
        Route {
            address: Addr::mock(1).into(),
            fee: Uint128::ZERO,
        },
    )
    .await
    .unwrap()
    .should_succeed();

    tprintln!("route setted!");

    let msg = process_terminal::block_search_message("Validator", "Waiting for").unwrap();
    tprintln!("msg: {}", msg);

    ch.send_remote(
        "user_1",
        Coin::new(DANGO_DENOM.clone(), 100).unwrap(),
        10,
        Addr::mock(2),
    )
    .await
    .unwrap()
    .should_succeed();

    tprintln!("Transfer remote broadcast success!");
    tprintln!("Send another transfer in 10 seconds...");

    std::thread::sleep(std::time::Duration::from_secs(10));

    ch.send_remote(
        "user_1",
        Coin::new(DANGO_DENOM.clone(), 200).unwrap(),
        10,
        Addr::mock(3),
    )
    .await
    .unwrap()
    .should_succeed();

    std::thread::sleep(std::time::Duration::from_secs(200));

    process_terminal::end_terminal();
}
