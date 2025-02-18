use {
    hyperlane_base::settings::{CheckpointSyncerConf, SignerConf},
    hyperlane_core::utils::hex_or_base58_to_h256,
    std::io::{BufRead, BufReader},
    utils::{
        agent::{Agent, AgentBuilder},
        constants::{CHAIN_ID, LOCALHOST, USER_2},
        dangod::{await_until_chain_start, dangod_reset, dangod_start},
        printer::Printer,
    },
};

pub mod utils;

// cargo test --package hyperlane-dango --test integration -- run_validator --exact --nocapture
#[tokio::test]
async fn run_validator() {
    dangod_reset();

    let dango = dangod_start();

    let printer = Printer::new();

    printer.set_dango(dango);

    await_until_chain_start(&printer, CHAIN_ID, LOCALHOST).await;

    printer.add_message("launching agent...");

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

    printer.set_agent(agent);

    printer.add_message("1");
    std::thread::sleep(std::time::Duration::from_secs(1));
    printer.add_message("2");
    std::thread::sleep(std::time::Duration::from_secs(1));
    printer.add_message("3");
    std::thread::sleep(std::time::Duration::from_secs(1));
    printer.add_message("4");
    std::thread::sleep(std::time::Duration::from_secs(1));
    printer.add_message("5");
    std::thread::sleep(std::time::Duration::from_secs(1));
    printer.add_message("6");
    std::thread::sleep(std::time::Duration::from_secs(1));
    printer.add_message("7");
}

// cargo test --package hyperlane-dango --test lol -- run_validator --exact --nocapture
#[test]
fn asd() {
    let printer = Printer::new();

    printer.add_message("1");
    std::thread::sleep(std::time::Duration::from_secs(1));
    printer.add_message("2");
    std::thread::sleep(std::time::Duration::from_secs(1));
    printer.add_message("3");
    std::thread::sleep(std::time::Duration::from_secs(1));
    printer.add_message("4");
    std::thread::sleep(std::time::Duration::from_secs(1));
    printer.add_message("5");
    std::thread::sleep(std::time::Duration::from_secs(1));
    printer.add_message("6");
    std::thread::sleep(std::time::Duration::from_secs(1));
    printer.add_message("7");
}
