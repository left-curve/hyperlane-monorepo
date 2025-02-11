use {
    super::DangoProvider,
    crate::{BlockOutcome, HyperlaneDangoResult, SearchTxOutcome},
    async_trait::async_trait,
    grug::{Addr, ContractInfo, Denom, Hash256, Signer, Uint128},
    serde::{de::DeserializeOwned, Serialize},
};

#[derive(Debug, Clone)]
pub struct GraphQlProvider {}

#[async_trait]
impl DangoProvider for GraphQlProvider {
    async fn get_block(&self, _height: Option<u64>) -> HyperlaneDangoResult<BlockOutcome> {
        unimplemented!()
    }

    async fn search_tx(&self, _hash: Hash256) -> HyperlaneDangoResult<SearchTxOutcome> {
        unimplemented!()
    }

    async fn balance(&self, _addr: Addr, _denom: Denom) -> HyperlaneDangoResult<Uint128> {
        unimplemented!()
    }

    async fn contract_info(&self, _addr: Addr) -> HyperlaneDangoResult<ContractInfo> {
        unimplemented!()
    }

    async fn query_wasm_smart<M, R>(
        &self,
        _contract: Addr,
        _msg: &M,
        _height: Option<u64>,
    ) -> HyperlaneDangoResult<R>
    where
        M: Serialize + Send + Sync,
        R: DeserializeOwned,
    {
        unimplemented!()
    }

    async fn send_message<S>(
        &self,
        _signer: &mut S,
        _msg: grug::Message,
    ) -> HyperlaneDangoResult<Hash256>
    where
        S: Signer + Send + Sync,
    {
        unimplemented!()
    }
}
