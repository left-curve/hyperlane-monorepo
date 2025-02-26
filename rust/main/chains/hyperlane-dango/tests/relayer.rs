use {
    process_terminal::{MessageSettings, ProcessSettings},
    std::process::{Child, Command, Stdio},
};

pub mod utils;

fn startup_chain(chain_name: &str, port: u16) -> Child {
    Command::new("docker")
        .args(&[
            "run",
            "--rm",
            "--name",
            chain_name,
            "-p",
            &format!("{}:26657", port),
            "dango",
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
    let d1 = startup_chain("dango1", 26657);
    let d2 = startup_chain("dango2", 36657);

    process_terminal::with_exit_callback(|| exit_process(&["dango1", "dango2"]));
    process_terminal::add_process("Dango1", d1, ProcessSettings::new(MessageSettings::Output))
        .unwrap();
    process_terminal::add_process("Dango2", d2, ProcessSettings::new(MessageSettings::Output))
        .unwrap();

    loop {
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    }
}
