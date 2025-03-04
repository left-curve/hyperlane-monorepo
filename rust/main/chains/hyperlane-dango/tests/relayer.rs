use {
    grug::btree_set,
    process_terminal::{KeyCode, MessageSettings, ProcessSettings, ScrollSettings},
    std::process::Command,
    utils::{
        agent::{Agent, AgentBuilder},
        dangod::DangodBuilder,
    },
};

pub mod utils;

fn exit_process(names: &[&str]) {
    println!("Exiting processes");
    for name in names {
        Command::new("docker")
            .args(&["kill", name])
            .output()
            .unwrap();
    }
}

#[tokio::test]
async fn relayer() {
    let (d1, d2) = try_start_test!(tokio::try_join!(
        DangodBuilder::new("dango1").start(),
        DangodBuilder::new("dango2")
            .with_hyperlane_domain(88888887)
            .with_rpc_port(36657)
            .start()
    ));

    process_terminal::add_process(
        "Dango1",
        d1.child,
        ProcessSettings::new(MessageSettings::Output),
    )
    .unwrap();
    process_terminal::add_process(
        "Dango2",
        d2.child,
        ProcessSettings::new(MessageSettings::Output),
    )
    .unwrap();

    process_terminal::with_exit_callback(|| exit_process(&["dango1", "dango2"]));

    let agent = AgentBuilder::new(Agent::Relayer)
        .with_origin_chain_name("dango1")
        .with_relay_chains(btree_set!("dango1", "dango2"))
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

    loop {
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    }
}
