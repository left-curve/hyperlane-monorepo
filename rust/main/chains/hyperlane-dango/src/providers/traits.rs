use {
    async_trait::async_trait,
    grug::{Addr, BlockInfo, ContractInfo, Hash256, Message, Signer, Tx, Uint128},
    serde::{de::DeserializeOwned, Serialize},
};

#[async_trait]
pub trait DangoProvider {
    type Error;

    async fn get_block(&self, height: Option<u64>) -> Result<BlockInfo, Self::Error>;

    async fn search_tx(&self, hash: Hash256) -> Result<Tx, Self::Error>;

    async fn query_balance(&self, hash: Addr, denom: &str) -> Result<Uint128, Self::Error>;

    async fn query_contract(&self, hash: Addr) -> Result<ContractInfo, Self::Error>;

    async fn query_wasm_smart<M, R>(
        &self,
        contract: Addr,
        msg: &M,
        height: Option<u64>,
    ) -> Result<R, Self::Error>
    where
        M: Serialize,
        R: DeserializeOwned;

    async fn send_message<S>(&self, signer: &mut S, msg: Message) -> Result<Hash256, Self::Error>
    where
        S: Signer;

    async fn send_messages<S>(
        &self,
        signer: &mut S,
        msgs: Vec<Message>,
    ) -> Result<Hash256, Self::Error>
    where
        S: Signer;
}
