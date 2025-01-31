use {
    grug::{Addr, Denom},
    hyperlane_dango::RpcProvider,
    std::{str::FromStr, sync::LazyLock},
    url::Url,
};

const RPC_URL: &str = "http://65.108.46.248:26657";

const EXISTING_CONTRACT: LazyLock<Addr> =
    LazyLock::new(|| Addr::from_str("0x2f3d763027f30db0250de65d037058c8bcbd3352").unwrap());
const NOT_EXISTING_CONTRACT: LazyLock<Addr> =
    LazyLock::new(|| Addr::from_str("0x929a99d0881f07e03d5f91b5ad2a1fc188f64ea1").unwrap());

const EXISTING_USER: LazyLock<Addr> =
    LazyLock::new(|| Addr::from_str("0xcf8c496fb3ff6abd98f2c2b735a0a148fed04b54").unwrap());
const NOT_EXISTING_USER: LazyLock<Addr> =
    LazyLock::new(|| Addr::from_str("0x384ba320f302804a0a03bfc8bb171f35d8b84f01").unwrap());

const EXISTING_COIN: LazyLock<Denom> = LazyLock::new(|| Denom::from_str("uusdc").unwrap());
const NOT_EXISTING_COIN: LazyLock<Denom> = LazyLock::new(|| Denom::from_str("abcde").unwrap());

#[tokio::test]
async fn rpc_test() {
    let client = RpcProvider::new(&Url::parse(RPC_URL).unwrap()).unwrap();

    // Get block.
    {
        // Get the latest block.
        let block = client.get_block(None).await.unwrap();
        // Wait some time before asking again.
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
        let new_block = client.get_block(None).await.unwrap();

        // Check that the new block is higher than the old block.
        assert!(block.block.header.height < new_block.block.header.height);

        // Get the block at a specific height.
        let height = 10;
        let block = client.get_block(Some(height)).await.unwrap();
        assert!(block.block.header.height.value() == height);
    }

    // Check if a contract exists.
    {
        assert!(client.is_contract(*EXISTING_CONTRACT).await.unwrap());
        assert!(!client.is_contract(*NOT_EXISTING_CONTRACT).await.unwrap());
    }

    // Get the balance of an address.
    {
        // Get the balance of a coin for an address.
        let balance = client
            .get_balance(*EXISTING_USER, EXISTING_COIN.clone())
            .await
            .unwrap();

        assert!(balance.denom == *EXISTING_COIN);
        assert!(balance.amount > 0.into());

        // Get the balance of a NOT existing coin for an address.
        let balance = client
            .get_balance(*EXISTING_USER, NOT_EXISTING_COIN.clone())
            .await
            .unwrap();
        assert!(balance.amount == 0.into());

        // Get the balance of a coin for a NOT existing address.
        let balance = client
            .get_balance(*NOT_EXISTING_USER, EXISTING_COIN.clone())
            .await
            .unwrap();
        assert!(balance.amount == 0.into());
    }

    // TODO add the test for tx.
}
