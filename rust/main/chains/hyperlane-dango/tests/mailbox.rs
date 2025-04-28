pub mod utils;

use {
    dango_types::{
        constants::DANGO_DENOM,
        warp::{ExecuteMsg, Route},
    },
    grug::{Addr, Coin, Message, ResultExt},
    hyperlane_core::{ReorgPeriod, H256},
    hyperlane_dango::DangoConvertor,
    std::{str::FromStr, sync::LazyLock},
    utils::{
        config::{ChainConfBuilder, DEFAULT_RPC_PORT},
        constants::EMPTY_METRICS,
        dango_builder::{kill_docker_processes, DangoBuilder},
        user::IntoSignerConf,
    },
};

const ORIGIN_DOMAIN: u32 = 1;
const DESTINATION_DOMAIN: u32 = 2;

// The recipient of the message is the contract address where the message is sent,
// so is the destination address set in the route.
const MESSAGE_RECIPIENT: LazyLock<Addr> =
    LazyLock::new(|| Addr::from_str("0xcf8c496fb3ff6abd98f2c2b735a0a148fed04b54").unwrap());

#[tokio::test]
async fn mailbox() {
    let docker_name = "dango";
    let (mut chain_helper, _) = DangoBuilder::new(docker_name)
        .with_rpc_port(DEFAULT_RPC_PORT)
        .with_hyperlane_domain(ORIGIN_DOMAIN)
        .start()
        .await
        .unwrap();

    process_terminal::with_exit_callback(|| kill_docker_processes(&[docker_name]));

    let user1 = chain_helper.accounts.get("user_1").unwrap();
    // let user1_address = user1.address.clone();

    let test_suite = ChainConfBuilder::new(chain_helper.chain_id.clone())
        .with_default_rpc_provider()
        .with_signer(user1.as_signer_conf())
        .build()
        .await;

    let chain_conf = test_suite.chain_conf;

    // Build the merkle tree hook.
    let mailbox = chain_conf.build_mailbox(&EMPTY_METRICS).await.unwrap();

    // Build merkle tree hook indexer.
    let mailbox_indexer = chain_conf
        .build_message_indexer(&EMPTY_METRICS, false)
        .await
        .unwrap();

    // Add the route for the destination domain.
    chain_helper
        .set_route(
            DANGO_DENOM.clone(),
            DESTINATION_DOMAIN,
            Route {
                address: MESSAGE_RECIPIENT.clone().into(),
                fee: 0.into(),
            },
        )
        .await
        .unwrap()
        .should_succeed();

    // Retrieve the data updated to the last block.
    let reorg_period = ReorgPeriod::None;

    // Assert mailbox is empty.
    assert_eq!(
        mailbox.count(&reorg_period).await.unwrap(),
        0,
        "Mailbox is not empty."
    );

    // Create a Transfer remote message.
    let msg = Message::execute(
        test_suite.warp_address,
        &ExecuteMsg::TransferRemote {
            destination_domain: DESTINATION_DOMAIN,
            recipient: Addr::mock(1).into(),
            metadata: None,
        },
        Coin::new(DANGO_DENOM.clone(), 100).unwrap(),
    )
    .unwrap();

    // Send first message.
    {
        // Get the block before the message is sent.
        let block_before = mailbox_indexer.get_finalized_block_number().await.unwrap();

        let res = test_suite
            .dango_provider
            .send_message_and_find(msg.clone(), None)
            .await
            .unwrap();

        assert!(res.executed, "Failed to send the message.");

        // Get the block after the message is sent.
        let block_after = mailbox_indexer.get_finalized_block_number().await.unwrap();

        // Check the mailbox count is 1.
        assert_eq!(
            mailbox.count(&reorg_period).await.unwrap(),
            1,
            "Mailbox count is not correct."
        );

        // Retrieve the logs in the range.
        let logs = mailbox_indexer
            .fetch_logs_in_range(block_before..=block_after)
            .await
            .unwrap();

        // Check the log is inserted.
        assert_eq!(logs.len(), 1);

        // Check the hyperlane message.
        let log = logs[0].0.inner();

        assert!(log.nonce == 0, "The nonce is not correct.");
        assert!(
            log.origin == ORIGIN_DOMAIN,
            "The origin domain is not correct."
        );
        assert!(
            log.destination == DESTINATION_DOMAIN,
            "The destination domain is not correct."
        );
        assert!(
            log.sender == chain_helper.cfg.addresses.warp.convert(),
            "The sender is not correct: found {:?}, expected {:?}",
            log.sender,
            DangoConvertor::<H256>::convert(chain_helper.cfg.addresses.warp)
        );
        assert!(
            log.recipient == MESSAGE_RECIPIENT.clone().convert(),
            "The recipient is not correct: found {:?}, expected {:?}",
            log.recipient,
            DangoConvertor::<H256>::convert(MESSAGE_RECIPIENT.clone())
        );
    }

    // Send second message.
    {
        // Get the block before the message is sent.
        let block_before = mailbox_indexer.get_finalized_block_number().await.unwrap();

        let res = test_suite
            .dango_provider
            .send_message_and_find(msg, None)
            .await
            .unwrap();

        assert!(res.executed, "Failed to send the message.");

        // Get the block after the message is sent.
        let block_after = mailbox_indexer.get_finalized_block_number().await.unwrap();

        // Check the mailbox count is 2.
        assert_eq!(
            mailbox.count(&reorg_period).await.unwrap(),
            2,
            "Mailbox count is not correct."
        );

        // Retrieve the logs in the range.
        let logs = mailbox_indexer
            .fetch_logs_in_range(block_before..=block_after)
            .await
            .unwrap();

        assert_eq!(logs.len(), 1);
    }
}
