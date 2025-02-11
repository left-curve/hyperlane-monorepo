use {
    crate::{BlockOutcome, HyperlaneDangoError, SearchTxOutcome},
    async_trait::async_trait,
    grug::{Addr, ContractInfo, Denom, Hash256, Message, Signer, Uint128},
    serde::{de::DeserializeOwned, Serialize},
};

#[async_trait]
pub trait DangoProvider {
    type Error: Send + Sync + Into<HyperlaneDangoError>;

    /// Get block info for a given block height. If block height is None, return the latest block.
    async fn get_block(&self, height: Option<u64>) -> Result<BlockOutcome, Self::Error>;

    /// Get transaction info for a given transaction hash.
    async fn search_tx(&self, hash: Hash256) -> Result<SearchTxOutcome, Self::Error>;

    /// Get the balance of an address for a given denom.
    async fn balance(&self, addr: Addr, denom: Denom) -> Result<Uint128, Self::Error>;

    /// Get the contract info for a given contract address.
    async fn contract_info(&self, addr: Addr) -> Result<ContractInfo, Self::Error>;

    /// Query a wasm smart contract.
    async fn query_wasm_smart<M, R>(
        &self,
        contract: Addr,
        msg: &M,
        height: Option<u64>,
    ) -> Result<R, Self::Error>
    where
        M: Serialize + Send + Sync,
        R: DeserializeOwned;

    /// Sign and broadcast a message.
    async fn send_message<S>(&self, signer: &mut S, msg: Message) -> Result<Hash256, Self::Error>
    where
        S: Signer + Send + Sync;
}
