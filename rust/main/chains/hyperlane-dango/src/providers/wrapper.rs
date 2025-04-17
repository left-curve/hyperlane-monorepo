use {
    async_trait::async_trait,
    grug::{
        Binary, BlockClient, BroadcastClient, BroadcastTxOutcome, Query, QueryClient,
        QueryResponse, SearchTxClient, Tx, TxOutcome, UnsignedTx,
    },
    std::{fmt::Debug, ops::Deref, sync::Arc},
};

pub trait ClientRef<E>:
    BroadcastClient<Error = E>
    + QueryClient<Error = E, Proof = grug_jmt::Proof>
    + SearchTxClient<Error = E>
    + BlockClient<Error = E>
    + Debug
{
}

impl<T, E> ClientRef<E> for T where
    T: BroadcastClient<Error = E>
        + QueryClient<Error = E, Proof = grug_jmt::Proof>
        + SearchTxClient<Error = E>
        + BlockClient<Error = E>
        + Debug
{
}

#[derive(Debug, Clone)]
pub struct Wrapper {
    pub client: Arc<dyn ClientRef<anyhow::Error>>,
}

#[async_trait]
impl QueryClient for Wrapper {
    type Error = anyhow::Error;
    type Proof = grug_jmt::Proof;

    async fn query_app(
        &self,
        query: Query,
        height: Option<u64>,
    ) -> Result<QueryResponse, Self::Error> {
        self.client.query_app(query, height).await
    }

    async fn query_store(
        &self,
        key: Binary,
        height: Option<u64>,
        prove: bool,
    ) -> Result<(Option<Binary>, Option<Self::Proof>), Self::Error> {
        self.client.query_store(key, height, prove).await
    }

    async fn simulate(&self, tx: UnsignedTx) -> Result<TxOutcome, Self::Error> {
        self.client.simulate(tx).await
    }
}

#[async_trait]
impl BroadcastClient for Wrapper {
    type Error = anyhow::Error;
    async fn broadcast_tx(&self, tx: Tx) -> Result<BroadcastTxOutcome, Self::Error> {
        self.client.broadcast_tx(tx).await
    }
}

impl Deref for Wrapper {
    type Target = dyn ClientRef<anyhow::Error>;

    fn deref(&self) -> &Self::Target {
        self.client.as_ref()
    }
}
