use hyperlane_base::settings::{CheckpointSyncerConf, SignerConf};
use hyperlane_core::utils::hex_or_base58_to_h256;
use utils::{
    agent::{Agent, AgentBuilder},
    constants::USER_1,
};

pub mod utils;

#[test]
fn run_validator() {
    AgentBuilder::new(Agent::Validator)
        .with_origin_chain_name("dango")
        .with_checkpoint_syncer(CheckpointSyncerConf::LocalStorage {
            path: "dango_1".into(),
        })
        .with_validator_signer(SignerConf::HexKey {
            key: hex_or_base58_to_h256("0x76e21577e7df18de93bbe82779bf3a16b2bacfd9").unwrap(),
        })
        .with_chain_signer("dango", USER_1.clone().into())
        .launch();
}
