use src::ChainConfBuilder;

mod src;

#[tokio::test]
async fn validator_announce() {
    let chain_conf = ChainConfBuilder::new()
        .with_default_rpc_provider()
        .build()
        .await;

    // let metrics = chain_conf.metr
    // let va = chain_conf
    //     .build_validator_announce(&chain_conf.metrics_conf)
    //     .await;
}
