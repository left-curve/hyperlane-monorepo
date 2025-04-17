use {
    hyperlane_base::settings::{parser::h_eth::SingletonSigner, SignerConf},
    hyperlane_core::{Announcement, HyperlaneSigner, HyperlaneSignerExt, H256},
    hyperlane_dango::DangoConvertor,
    utils::{
        config::{ChainConfBuilder, DEFAULT_RPC_PORT},
        constants::EMPTY_METRICS,
        dango_builder::{kill_docker_processes, DangoBuilder},
        user::IntoSignerConf,
    },
};

pub mod utils;

#[tokio::test]
async fn validator() {
    let docker_name = "dango";
    let (chain_helper, _) = DangoBuilder::new(docker_name)
        .with_rpc_port(DEFAULT_RPC_PORT)
        .start()
        .await
        .unwrap();

    let user1 = chain_helper
        .accounts
        .get("user_1")
        .unwrap()
        .as_signer_conf();

    let user_key = if let SignerConf::Dango { key, .. } = user1 {
        key
    } else {
        panic!("Failed to convert signer");
    };

    let mut test_suite = ChainConfBuilder::new(chain_helper.chain_id.clone())
        .with_default_rpc_provider()
        .with_signer(user1)
        .build()
        .await;

    let chain_conf = test_suite.chain_conf;

    test_suite.dango_provider.connection_conf.chain_id = chain_helper.chain_id.clone();

    // Create a validator announce instance.
    let va = chain_conf
        .build_validator_announce(&EMPTY_METRICS)
        .await
        .unwrap();

    // Assert that the address is correct.
    assert_eq!(va.address(), chain_conf.addresses.validator_announce);

    // Create a signer instance for the validator.
    let (singner_handler, signer) = SingletonSigner::new(
        SignerConf::HexKey {
            key: user_key.convert(),
        }
        .build()
        .await
        .unwrap(),
    );

    // Run the signer instance.
    tokio::spawn(async move {
        singner_handler.run().await;
    });

    // Announce the validator.
    let storage_location = "Test/storage/location".to_string();
    let announcement = Announcement {
        validator: signer.eth_address(),
        mailbox_address: chain_conf.addresses.mailbox,
        mailbox_domain: chain_conf.domain.id(),
        storage_location: storage_location.clone(),
    };

    let signed_announcement = signer.sign(announcement).await.unwrap();

    let res = va.announce(signed_announcement.clone()).await.unwrap();
    assert!(
        res.executed,
        "Failed to announce validator, hash: {}",
        res.transaction_id
    );

    // Check that the announcement was written on chain.
    let validators: [H256; 1] = [signer.eth_address().into()];
    if let Some(announcement_location) = va
        .get_announced_storage_locations(&validators)
        .await
        .unwrap()
        .first()
    {
        assert!(
            announcement_location.len() == 1,
            "There should only be 1 storage location"
        );

        assert!(
            announcement_location.contains(&storage_location),
            "Storage was not announced correctly"
        );
    } else {
        panic!("No announcement location found");
    }

    // Announce a new storage location.
    let storage_location2 = "Test2/storage2/location2".to_string();
    let announcement2 = Announcement {
        validator: signer.eth_address(),
        mailbox_address: chain_conf.addresses.mailbox,
        mailbox_domain: chain_conf.domain.id(),
        storage_location: storage_location2.clone(),
    };

    let signed_announcement2 = signer.sign(announcement2).await.unwrap();

    let res = va.announce(signed_announcement2).await.unwrap();
    assert!(
        res.executed,
        "Failed to announce validator, hash: {}",
        res.transaction_id
    );

    if let Some(announcement_location) = va
        .get_announced_storage_locations(&validators)
        .await
        .unwrap()
        .first()
    {
        assert!(
            announcement_location.len() == 2,
            "There should only be 1 storage location"
        );

        assert!(
            announcement_location[0] == storage_location,
            "Wrong storage location on first index, expected: {}, got: {}",
            storage_location,
            announcement_location[0]
        );

        assert!(
            announcement_location[1] == storage_location2,
            "Wrong storage location on second index, expected: {}, got: {}",
            storage_location2,
            announcement_location[1]
        );
    } else {
        panic!("No announcement location found");
    }

    kill_docker_processes(&[docker_name]);
}
