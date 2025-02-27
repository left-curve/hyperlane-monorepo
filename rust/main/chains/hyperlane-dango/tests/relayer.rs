use {
    grug::btree_set,
    process_terminal::{KeyCode, MessageSettings, ProcessSettings, ScrollSettings},
    std::process::{Child, Command, Stdio},
    utils::agent::{Agent, AgentBuilder},
};

pub mod utils;

fn startup_chain(chain_name: &str, port: u16, hyperlane_domain: u32) -> Child {
    Command::new("docker")
        .args(&[
            "run",
            "--rm",
            "--name",
            chain_name,
            "-p",
            &format!("{port}:26657"),
            "dango2",
            &format!("--hyperlane_domain {hyperlane_domain}"),
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap()
}

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
    startup_chain("dango1", 26657, 88888887);
    startup_chain("dango2", 36657, 88888886);

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
