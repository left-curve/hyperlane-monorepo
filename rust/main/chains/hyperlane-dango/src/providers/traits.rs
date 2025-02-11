use {
    crate::{BlockOutcome, HyperlaneDangoResult, SearchTxOutcome},
    async_trait::async_trait,
    grug::{Addr, ContractInfo, Denom, Hash256, Message, Signer, Uint128},
    serde::{de::DeserializeOwned, Serialize},
};

#[async_trait]
pub trait DangoProvider {
    /// Get block info for a given block height. If block height is None, return the latest block.
    async fn get_block(&self, height: Option<u64>) -> HyperlaneDangoResult<BlockOutcome>;

    /// Get transaction info for a given transaction hash.
    async fn search_tx(&self, hash: Hash256) -> HyperlaneDangoResult<SearchTxOutcome>;

    /// Get the balance of an address for a given denom.
    async fn balance(&self, addr: Addr, denom: Denom) -> HyperlaneDangoResult<Uint128>;

    /// Get the contract info for a given contract address.
    async fn contract_info(&self, addr: Addr) -> HyperlaneDangoResult<ContractInfo>;

    /// Query a wasm smart contract.
    async fn query_wasm_smart<M, R>(
        &self,
        contract: Addr,
        msg: &M,
        height: Option<u64>,
    ) -> HyperlaneDangoResult<R>
    where
        M: Serialize + Send + Sync,
        R: DeserializeOwned;

    /// Sign and broadcast a message.
    async fn send_message<S>(&self, signer: &mut S, msg: Message) -> HyperlaneDangoResult<Hash256>
    where
        S: Signer + Send + Sync;
}
