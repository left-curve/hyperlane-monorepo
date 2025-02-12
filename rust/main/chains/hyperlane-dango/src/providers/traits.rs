use {
    crate::{BlockOutcome, BlockResultOutcome, DangoResult, SearchTxOutcome},
    async_trait::async_trait,
    grug::{
        Addr, ContractInfo, Denom, GasOption, Hash256, Message, QueryRequest, Signer, TxOutcome,
        Uint128,
    },
    serde::{de::DeserializeOwned, Serialize},
};

#[async_trait]
pub trait DangoProviderInterface {
    /// Get block info for a given block height. If block height is None, return the latest block.
    async fn get_block(&self, height: Option<u64>) -> DangoResult<BlockOutcome>;

    /// Get block result for a given block height. If block height is None, return the latest block.
    async fn get_block_result(&self, height: Option<u64>) -> DangoResult<BlockResultOutcome>;

    /// Get transaction info for a given transaction hash.
    async fn search_tx(&self, hash: Hash256) -> DangoResult<SearchTxOutcome>;

    /// Get the balance of an address for a given denom.
    async fn balance(&self, addr: Addr, denom: Denom) -> DangoResult<Uint128>;

    /// Get the contract info for a given contract address.
    async fn contract_info(&self, addr: Addr) -> DangoResult<ContractInfo>;

    /// Query a wasm smart contract.
    async fn query_wasm_smart<R>(
        &self,
        contract: Addr,
        req: R,
        height: Option<u64>,
    ) -> DangoResult<R::Response>
    where
        R: QueryRequest + Send + Sync + 'static,
        R::Message: Serialize + Send + Sync + 'static,
        R::Response: DeserializeOwned;

    /// Sign and broadcast a message.
    async fn send_message<S>(
        &self,
        signer: &mut S,
        msg: Message,
        gas: GasOption,
    ) -> DangoResult<Hash256>
    where
        S: Signer + Send + Sync;

    async fn simulate_message<S>(&self, signer: &S, msg: Message) -> DangoResult<TxOutcome>
    where
        S: Signer + Send + Sync;
}
