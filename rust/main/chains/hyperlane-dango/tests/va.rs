use {
    bip32::{Language, Mnemonic},
    dango_client::{SigningKey, SingleSigner},
    grug::{Addr, HexByteArray},
    hyperlane_base::{
        settings::{parser::h_eth::SingletonSigner, SignerConf},
        CoreMetrics,
    },
    hyperlane_core::{Announcement, HyperlaneSigner, HyperlaneSignerExt, H256},
    std::str::FromStr,
    utils::{config::ChainConfBuilder, constants::COIN_TYPE},
};

pub mod utils;

pub const MNEMONIC: &str = "impulse youth electric wink tomorrow fruit squirrel practice effort mimic leave year visual calm surge system census tower involve wild symbol coral purchase uniform";
pub const ADDRESS: &str = "0xa4f1194e28a176c15ec2fe499fec873ce4756f14";
pub const USERNAME: &str = "user_1";

#[tokio::test]
async fn validator() {
    let mnemonic = Mnemonic::new(MNEMONIC, Language::English).unwrap();
    let singing_key = SigningKey::from_mnemonic(&mnemonic, COIN_TYPE).unwrap();
    let key = HexByteArray::from(singing_key.private_key());
    let user = SingleSigner::new(
        USERNAME,
        Addr::from_str(ADDRESS).unwrap(),
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
            key: singing_key.private_key().into(),
        }
        .build()
        .await
        .unwrap(),
    );

    // Run the signer instance.
    tokio::spawn(async move {
        singner_handler.run().await;
    });

    // Assert that there is no announcement_location for this validator.
    let validators: [H256; 1] = [signer.eth_address().into()];
    if let Some(_) = va
        .get_announced_storage_locations(&validators)
        .await
        .unwrap()
        .first()
    {
        panic!("There should be no announcement onchain.");
    };

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

#[tokio::test]
async fn private_key() {
    let mnemonic = Mnemonic::new(MNEMONIC, Language::English).unwrap();
    let singing_key = SigningKey::from_mnemonic(&mnemonic, COIN_TYPE).unwrap();
    let key = HexByteArray::from(singing_key.private_key());

    println!("{key}")
}
