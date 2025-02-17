use hyperlane_base::agent_main;

#[tokio::test]
async fn integration() {
    agent_main::<validator::Validator>().await.unwrap();
}
