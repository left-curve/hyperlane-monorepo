use {
    hyperlane_base::{
        settings::{parser::h_eth::SingletonSigner, SignerConf},
        CoreMetrics,
    },
    hyperlane_core::{Announcement, HyperlaneSigner, HyperlaneSignerExt, H256},
    hyperlane_dango::DangoConvertor,
    utils::{config::ChainConfBuilder, constants::USER_1},
};

pub mod utils;

#[tokio::test]
async fn validator() {
    let test_suite = ChainConfBuilder::new()
        .with_default_rpc_provider()
        .with_signer(USER_1.to_owned().into())
        .build()
        .await;

    let chain_conf = test_suite.chain_conf;

    // Create a validator announce instance.
    let va = chain_conf
        .build_validator_announce(
            &CoreMetrics::new("va", 9090, prometheus::Registry::new()).unwrap(),
        )
        .await
        .unwrap();

    // Assert that the address is correct.
    assert_eq!(va.address(), chain_conf.addresses.validator_announce);

    // Create a signer instance for the validator.
    let (singner_handler, signer) = SingletonSigner::new(
        SignerConf::HexKey {
            key: USER_1.sk.convert(),
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

    // Announce the validator.
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

    // Announce the validator again (works but still only 1 storage location).
    let res = va.announce(signed_announcement).await.unwrap();
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

    // Announce the validator.
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
}
