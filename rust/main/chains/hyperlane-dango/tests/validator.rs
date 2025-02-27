use {
    dango_types::{
        config::AppConfig,
        constants::DANGO_DENOM,
        warp::{self, Route},
    },
    grug::{Addr, Coins, Defined, GasOption, Message, NumberConst, Uint128},
    hyperlane_base::settings::{CheckpointSyncerConf, SignerConf},
    hyperlane_core::utils::hex_or_base58_to_h256,
    hyperlane_dango::DangoProviderInterface,
    process_terminal::{tprintln, KeyCode, MessageSettings, ProcessSettings, ScrollSettings},
    utils::{
        agent::{Agent, AgentBuilder},
        constants::USER_2,
        dangod::{DangodBuilder, DangodEnv},
    },
};

pub mod utils;

#[tokio::test]
async fn run_validator() {
    let DangodEnv {
        child,
        mut accounts,
        client,
    } = DangodBuilder::new().start().await.unwrap();

    process_terminal::add_process(
        "Dango",
        child,
        ProcessSettings::new(MessageSettings::Output),
    )
    .unwrap();

    let child = AgentBuilder::new(Agent::Validator)
        .with_origin_chain_name("dango1")
        .with_checkpoint_syncer(CheckpointSyncerConf::LocalStorage {
            path: "dango_1".into(),
        })
        .with_validator_signer(SignerConf::HexKey {
            key: hex_or_base58_to_h256("0x76e21577e7df18de93bbe82779bf3a16b2bacfd9").unwrap(),
        })
        .with_chain_signer("dango1", USER_2.clone().into())
        .launch();

    process_terminal::add_process(
        "Validator",
        child,
        ProcessSettings::new_with_scroll(
            MessageSettings::All,
            ScrollSettings::enable(KeyCode::Up, KeyCode::Down),
        ),
    )
    .unwrap();

    let app_cfg: AppConfig = client.query_app_config().await.unwrap();

    // Set route
    let tx = client
        .send_message(
            &mut accounts["owner"],
            Message::execute(
                app_cfg.addresses.warp,
                &warp::ExecuteMsg::SetRoute {
                    denom: DANGO_DENOM.clone(),
                    destination_domain: 10,
                    route: Route {
                        address: Addr::mock(1).into(),
                        fee: Uint128::ZERO,
                    },
                },
                Coins::default(),
            )
            .unwrap(),
            GasOption::Predefined {
                gas_limit: 10_000_000,
            },
        )
        .await
        .unwrap();

    assert!(tx.code.is_ok(), "tx failed: {:?}", tx);

    tprintln!("route setted!");

    let msg = process_terminal::block_search_message("Validator", "Waiting for").unwrap();
    tprintln!("msg: {}", msg);

    let tx = loop {
        match client
            .send_message(
                &mut accounts["user_1"],
                Message::execute(
                    app_cfg.addresses.warp,
                    &warp::ExecuteMsg::TransferRemote {
                        destination_domain: 10,
                        recipient: Addr::mock(2).into(),
                        metadata: None,
                    },
                    Coins::one(DANGO_DENOM.clone(), 100).unwrap(),
                )
                .unwrap(),
                GasOption::Predefined {
                    gas_limit: 10_000_000,
                },
            )
            .await
        {
            Ok(tx) => break tx,
            Err(_err) => {
                tprintln!("Transfer remote broadcast fail!");
                std::thread::sleep(std::time::Duration::from_secs(2));
                let sequence = accounts["user_1"].nonce.into_inner() - 1;
                *&mut accounts["user_1"].nonce = Defined::new(sequence);
            }
        }
    };

    assert!(tx.code.is_ok(), "tx failed: {:?}", tx);

    if tx.code.is_err() {
        tprintln!("tx failed: {:?}", tx);
    };

    tprintln!("Transfer remote broadcast success!");
    tprintln!("Send another transfer in 10 seconds...");

    std::thread::sleep(std::time::Duration::from_secs(10));

    let tx = loop {
        match client
            .send_message(
                &mut accounts["user_1"],
                Message::execute(
                    app_cfg.addresses.warp,
                    &warp::ExecuteMsg::TransferRemote {
                        destination_domain: 10,
                        recipient: Addr::mock(3).into(),
                        metadata: None,
                    },
                    Coins::one(DANGO_DENOM.clone(), 200).unwrap(),
                )
                .unwrap(),
                GasOption::Predefined {
                    gas_limit: 10_000_000,
                },
            )
            .await
        {
            Ok(tx) => break tx,
            Err(_err) => {
                tprintln!("Transfer remote broadcast fail!");
                std::thread::sleep(std::time::Duration::from_secs(2));
                let sequence = accounts["user_1"].nonce.into_inner() - 1;
                *&mut accounts["user_1"].nonce = Defined::new(sequence);
            }
        }
    };

    assert!(tx.code.is_ok(), "tx failed: {:?}", tx);

    if tx.code.is_err() {
        tprintln!("tx failed: {:?}", tx);
    };

    std::thread::sleep(std::time::Duration::from_secs(200));

    process_terminal::end_terminal();
}

#[tokio::test]
async fn relayer() {
    
}