pub mod utils;

use {
    dango_types::{
        constants::DANGO_DENOM,
        warp::{ExecuteMsg, Route},
    },
    grug::{Coin, Message, ResultExt},
    hyperlane_core::ReorgPeriod,
    utils::{
        config::{ChainConfBuilder, DANGO_DOMAIN, DEFAULT_RPC_PORT},
        constants::{EMPTY_METRICS, OWNER},
        dango_builder::DangoBuilder,
        user::IntoSignerConf,
    },
};

pub const DESTINATION_DOMAIN: u32 = 1;

#[tokio::test]
async fn merkle_tree() {
    let docker_name = "dango";
    let (mut chain_helper, _) = DangoBuilder::new(docker_name)
        .with_rpc_port(DEFAULT_RPC_PORT)
        .start()
        .await
        .unwrap();

    let user1 = chain_helper
        .accounts
        .get("user_1")
        .unwrap()
        .as_signer_conf();

    // let user_key = if let SignerConf::Dango { key, .. } = user1 {
    //     key
    // } else {
    //     panic!("Failed to convert signer");
    // };

    let test_suite = ChainConfBuilder::new(chain_helper.chain_id.clone())
        .with_default_rpc_provider()
        .with_signer(user1)
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
    chain_helper
        .set_route(
            DANGO_DENOM.clone(),
            DESTINATION_DOMAIN,
            Route {
                // The address is not important for the test.
                address: OWNER.address.into(),
                fee: 0.into(),
            },
        )
        .await
        .unwrap()
        .should_succeed();

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
            recipient: OWNER.address.into(),
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
