pub mod utils;

use {
    bip32::{Language, Mnemonic},
    dango_client::{SigningKey, SingleSigner},
    dango_types::warp::{ExecuteMsg, Route},
    grug::{Addr, Coin, Coins, Denom, HexByteArray, Message},
    hyperlane_base::settings::SignerConf,
    hyperlane_core::ReorgPeriod,
    std::{str::FromStr, sync::LazyLock},
    utils::config::{ChainConfBuilder, DANGO_DOMAIN, EMPTY_METRICS},
};

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
        .build_merkle_tree_hook(&EMPTY_METRICS)
        .await
        .unwrap();

    // Build merkle tree hook indexer.
    let merkle_tree_indexer = chain_conf
        .build_merkle_tree_hook_indexer(&EMPTY_METRICS, false)
        .await
        .unwrap();

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

    // Retrieve the data updated to the last block.
    let reorg_period = ReorgPeriod::None;

    // Create a local tree from chain.
    let mut tree = merkle_tree.tree(&reorg_period).await.unwrap();

    // Get the count of the tree.
    let count = merkle_tree.count(&reorg_period).await.unwrap();
    assert_eq!(count as usize, tree.count());

    // Get last checkpoint.
    let checkpoint = merkle_tree.latest_checkpoint(&reorg_period).await.unwrap();
    if tree.count() > 0 {
        assert_eq!(
            checkpoint.index as usize,
            tree.count() - 1,
            "The index is not correct."
        );
    } else {
        assert_eq!(checkpoint.index as usize, 0, "The index is not correct.");
    }
    assert_eq!(checkpoint.root, tree.root(), "Root tree does not match");
    assert_eq!(
        checkpoint.merkle_tree_hook_address,
        merkle_tree.address(),
        "Merkle tree hook address does not match"
    );
    assert_eq!(
        checkpoint.mailbox_domain,
        DANGO_DOMAIN.id(),
        "Mailbox domain does not match"
    );

    // Send a message warp contract to add to merkle tree onchain.
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

    // Get the block before the message is sent.
    let block_before = merkle_tree_indexer
        .get_finalized_block_number()
        .await
        .unwrap();

    let res = test_suite
        .dango_provider
        .send_message_and_find(msg, None)
        .await
        .unwrap();

    assert!(res.executed, "Failed to send the message.");

    // Get the block after the message is sent.
    let block_after = merkle_tree_indexer
        .get_finalized_block_number()
        .await
        .unwrap();

    // Retrieve the logs in the range.
    let logs = merkle_tree_indexer
        .fetch_logs_in_range(block_before..=block_after)
        .await
        .unwrap();

    // Check the log is inserted.
    assert_eq!(logs.len(), 1);

    // Update local tree with the new message.
    for (log, _) in logs {
        tree.ingest(log.inner().message_id());
    }

    // Retrieve the tree onchain.
    let tree_onchain = merkle_tree.tree(&reorg_period).await.unwrap();

    // Check the local tree is consistent with the onchain tree.
    assert_eq!(
        tree, tree_onchain,
        "The local tree is not consistent with the onchain tree."
    );

    //TODO: test fetch_logs_by_tx_hash for indexer
}
