use {
    super::DangoProviderInterface,
    crate::{BlockOutcome, BlockResultOutcome, DangoResult, SearchTxOutcome},
    async_trait::async_trait,
    grug::{
        Addr, ContractInfo, Denom, GasOption, Hash256, Message, QueryRequest, Signer, TxOutcome,
        Uint128,
    },
    serde::{de::DeserializeOwned, Serialize},
};

#[derive(Debug, Clone)]
pub struct GraphQlProvider {}

#[async_trait]
impl DangoProviderInterface for GraphQlProvider {
    async fn get_block(&self, _height: Option<u64>) -> DangoResult<BlockOutcome> {
        unimplemented!()
    }

    async fn get_block_result(&self, _height: Option<u64>) -> DangoResult<BlockResultOutcome> {
        unimplemented!()
    }

    async fn search_tx(&self, _hash: Hash256) -> DangoResult<SearchTxOutcome> {
        unimplemented!()
    }

    async fn balance(&self, _addr: Addr, _denom: Denom) -> DangoResult<Uint128> {
        unimplemented!()
    }

    async fn contract_info(&self, _addr: Addr) -> DangoResult<ContractInfo> {
        unimplemented!()
    }

    async fn query_wasm_smart<R>(
        &self,
        _contract: Addr,
        _req: R,
        _height: Option<u64>,
    ) -> DangoResult<R::Response>
    where
        R: QueryRequest + Send + Sync + 'static,
        R::Message: Serialize + Send + Sync + 'static,
        R::Response: DeserializeOwned,
    {
        unimplemented!()
    }

    async fn query_app_config<T>(&self) -> DangoResult<T>
    where
        T: DeserializeOwned,
    {
        unimplemented!()
    }

    async fn send_message<S>(
        &self,
        _signer: &mut S,
        _msg: grug::Message,
        _gas: GasOption,
    ) -> DangoResult<Hash256>
    where
        S: Signer + Send + Sync,
    {
        unimplemented!()
    }

    async fn simulate_message<S>(&self, _signer: &S, _msg: Message) -> DangoResult<TxOutcome>
    where
        S: Signer + Send + Sync,
    {
        unimplemented!()
    }
}
