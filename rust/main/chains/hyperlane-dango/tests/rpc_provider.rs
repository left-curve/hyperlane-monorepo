mod src;

use {
    grug::{Addr, Denom, ResultExt, SigningClient},
    hyperlane_dango::DangoProvider,
    src::constants::{
        EXISTING_COIN, EXISTING_CONTRACT, EXISTING_USER, NOT_EXISTING_COIN, NOT_EXISTING_CONTRACT,
        NOT_EXISTING_USER,
    },
    std::str::FromStr,
};

const RPC_URL: &str = "http://65.108.46.248:26657";

#[tokio::test]
async fn rpc_test() {
    let existing_contract = Addr::from_str(EXISTING_CONTRACT).unwrap();
    let not_existing_contract = Addr::from_str(NOT_EXISTING_CONTRACT).unwrap();
    let existing_user = Addr::from_str(EXISTING_USER).unwrap();
    let not_existing_user = Addr::from_str(NOT_EXISTING_USER).unwrap();
    let existing_coin = Denom::from_str(EXISTING_COIN).unwrap();
    let not_existing_coin = Denom::from_str(NOT_EXISTING_COIN).unwrap();

    let client = SigningClient::connect("dango", RPC_URL).unwrap();

    // Get block.
    {
        // Get the latest block.
        let block = client.get_block(None).await.unwrap();
        // Wait some time before asking again.
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
        let new_block = client.get_block(None).await.unwrap();

        // Check that the new block is higher than the old block.
        assert!(block.height < new_block.height);

        // Get the block at a specific height.
        let height = 10;
        let block = client.get_block(Some(height)).await.unwrap();
        assert!(block.height == height);
    }

    // Check if a contract exists.
    {
        client
            .contract_info(existing_contract)
            .await
            .should_succeed();

        client
            .contract_info(not_existing_contract)
            .await
            .should_fail();
    }

    // Get the balance of an address.
    {
        // Get the balance of a coin for an address.
        let balance = client
            .balance(existing_user, existing_coin.clone())
            .await
            .unwrap();

        assert!(balance > 0.into());

        // Get the balance of a NOT existing coin for an address.
        let balance = client
            .balance(existing_user, not_existing_coin)
            .await
            .unwrap();
        assert!(balance == 0.into());

        // Get the balance of a coin for a NOT existing address.
        let balance = client
            .balance(not_existing_user, existing_coin)
            .await
            .unwrap();
        assert!(balance == 0.into());
    }

    // TODO add the test for tx.
}
