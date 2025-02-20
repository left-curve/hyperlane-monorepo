use {
    core::panic,
    dango_types::{
        config::AppConfig,
        constants::DANGO_DENOM,
        warp::{self, Route},
    },
    grug::{Addr, Coins, Defined, GasOption, Message, NumberConst, Uint128},
    hyperlane_base::settings::{CheckpointSyncerConf, SignerConf},
    hyperlane_core::utils::hex_or_base58_to_h256,
    hyperlane_dango::DangoProviderInterface,
    utils::{
        agent::{Agent, AgentBuilder},
        constants::USER_2,
        dangod::{DangodBuilder, DangodEnv},
        printer::PRINTER,
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

    PRINTER.set_dango(child);

    let agent = AgentBuilder::new(Agent::Validator)
        .with_origin_chain_name("dango")
        .with_checkpoint_syncer(CheckpointSyncerConf::LocalStorage {
            path: "dango_1".into(),
        })
        .with_validator_signer(SignerConf::HexKey {
            key: hex_or_base58_to_h256("0x76e21577e7df18de93bbe82779bf3a16b2bacfd9").unwrap(),
        })
        .with_chain_signer("dango", USER_2.clone().into())
        .launch();

    PRINTER.set_agent(agent);

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

    dprintln!("route setted!");

    let msg = PRINTER.block_for_agent_submsg("Waiting for");

    dprintln!("msg: {}", msg);

    // sleep 2 s
    // std::thread::sleep(std::time::Duration::from_secs(80));

    // perform a hyperlane transfer

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
                dprintln!("Transfer remote broadcast fail!");
                std::thread::sleep(std::time::Duration::from_secs(2));
                let sequence = accounts["user_1"].nonce.into_inner() - 1;
                *&mut accounts["user_1"].nonce = Defined::new(sequence);
            }
        }
    };

    assert!(tx.code.is_ok(), "tx failed: {:?}", tx);

    if tx.code.is_err() {
        dprintln!("tx failed: {:?}", tx);
    };

    dprintln!("Transfer remote broadcast success!");

    std::thread::sleep(std::time::Duration::from_secs(200));

    // ratatui::restore();
}

#[tokio::test]
async fn asd() {
    let mut terminal = ratatui::init();

    for i in 0..2 {
        terminal
            .draw(|frame: &mut ratatui::Frame| {
                frame.render_widget(format!("hello world: {}", i), frame.area());
            })
            .unwrap();

        std::thread::sleep(std::time::Duration::from_secs(1));
    }

    panic!();
}

#[tokio::test]
async fn asd1() {
    dprintln!("hello");

    std::thread::sleep(std::time::Duration::from_secs(2));

    panic!("asd");
}
