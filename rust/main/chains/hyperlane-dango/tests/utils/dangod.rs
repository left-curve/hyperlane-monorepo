use {
    super::{printer::Printer, scope_child::ScopeChild},
    grug::SigningClient,
    std::process::{Command, Stdio},
};

pub fn dangod_reset() {
    Command::new("dangod").arg("reset").status().unwrap();
    Command::new("dangod")
        .arg("generate-static")
        .status()
        .unwrap();
    Command::new("dangod").arg("build").status().unwrap();
}

pub fn dangod_start() -> ScopeChild {
    ScopeChild::new(
        Command::new("dangod")
            .arg("start")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap(),
    )
}

pub async fn await_until_chain_start(printer: &Printer, chain_id: &str, endpoint: &str) {
    let provider = SigningClient::connect(chain_id, endpoint).unwrap();

    printer.add_message("waiting for chain to start...");

    loop {
        if provider.query_block(None).await.is_ok() {
            printer.add_message("chain started!");
            break;
        }
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
}
