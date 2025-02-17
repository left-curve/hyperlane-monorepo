use {
    bip32::{Language, Mnemonic},
    dango_client::{SigningKey, SingleSigner},
    dango_types::warp::{ExecuteMsg, Route},
    grug::{Addr, Coin, Coins, Denom, HexByteArray, Message},
    hyperlane_base::{settings::SignerConf, CoreMetrics},
    hyperlane_core::ReorgPeriod,
    std::{str::FromStr, sync::LazyLock},
    utils::config::ChainConfBuilder,
};
pub mod utils;

pub const MNEMONIC: &str = "junior fault athlete legal inject duty board school anger mesh humor file desk element ticket shop engine paper question love castle ghost bring discover";
pub const USER_ADDRESS: &str = "0xe430fa3a3f13c237fd2f20f8242857cef182b0bd";
pub const USERNAME: &str = "owner";
pub const COIN_TYPE: usize = 60;

pub const WARP: &str = "0x00d4f0a556bfeaa12e1451d74830cf483153af91";

pub const DANGO_DENOM: LazyLock<Denom> = LazyLock::new(|| Denom::from_str("udng").unwrap());
pub const DESTINATION_DOMAIN: u32 = 1;

#[tokio::test]
async fn merkle_tree() {
    let mnemonic = Mnemonic::new(MNEMONIC, Language::English).unwrap();
    let singing_key = SigningKey::from_mnemonic(&mnemonic, COIN_TYPE).unwrap();
    let key = HexByteArray::from(singing_key.private_key());
    let user = SingleSigner::new(
        USERNAME,
        Addr::from_str(USER_ADDRESS).unwrap(),
        singing_key.clone(),
    )
    .unwrap();

    let test_suite = ChainConfBuilder::new()
        .with_default_rpc_provider()
        .with_signer(SignerConf::Dango {
            username: user.username,
            key,
            address: user.address,
        })
        .build()
        .await;

    let chain_conf = test_suite.chain_conf;

    // Build the merkle tree hook.
    let merkle_tree = chain_conf
        .build_merkle_tree_hook(
            &CoreMetrics::new("merkle_tree", 9090, prometheus::Registry::new()).unwrap(),
        )
        .await
        .unwrap();

    // Test
    let reorg_period = ReorgPeriod::None;

    // Check count is 0.
    assert_eq!(merkle_tree.count(&reorg_period).await.unwrap(), 0);

    // Add the route for the destination domain.
    let msg = Message::execute(
        test_suite.warp_address,
        &ExecuteMsg::SetRoute {
            denom: DANGO_DENOM.clone(),
            destination_domain: DESTINATION_DOMAIN,
            route: Route {
                // The address is not important for the test.
                address: Addr::from_str(USER_ADDRESS).unwrap().into(),
                fee: 0.into(),
            },
        },
        Coins::new(),
    )
    .unwrap();

    let res = test_suite
        .dango_provider
        .send_message_and_find(msg, None)
        .await
        .unwrap();

    assert!(res.executed, "Failed to set the route.");

    // Add a new message.
    let msg = Message::execute(
        test_suite.warp_address,
        &ExecuteMsg::TransferRemote {
            destination_domain: 1,
            recipient: Addr::from_str(USER_ADDRESS).unwrap().into(),
            metadata: None,
        },
        Coin::new(DANGO_DENOM.clone(), 100).unwrap(),
    )
    .unwrap();

    let res = test_suite
        .dango_provider
        .send_message_and_find(msg, None)
        .await
        .unwrap();

    assert!(res.executed, "Failed to send the message.");

    // Check count is 1.
    assert_eq!(merkle_tree.count(&reorg_period).await.unwrap(), 1);
}
