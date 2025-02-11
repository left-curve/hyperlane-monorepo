use {
    super::DangoProvider,
    crate::{BlockOutcome, DangoResult, SearchTxOutcome, TryHashConvertor},
    async_trait::async_trait,
    grug::{
        Addr, ContractInfo, Denom, GasOption, Hash256, JsonDeExt, Message, Signer, SigningClient,
        Tx, TxOutcome, Uint128,
    },
    serde::{de::DeserializeOwned, Serialize},
    tendermint::abci::Code,
};

const GAS_OPTION_SCALE: f64 = 1.2;
const GAS_OPTION_FLAT_INCREASE: u64 = 100_000;

#[async_trait]
impl DangoProvider for SigningClient {
    async fn get_block(&self, height: Option<u64>) -> DangoResult<BlockOutcome> {
        let response = self.query_block(height).await?;

        Ok(BlockOutcome {
            hash: response.block_id.hash.try_convert()?,
            height: response.block.header.height.value(),
            timestamp: response.block.header.time.unix_timestamp() as u64,
            txs: response
                .block
                .data
                .into_iter()
                .map(|tx| tx.deserialize_json())
                .collect::<Result<_, _>>()?,
        })
    }

    async fn search_tx(&self, hash: Hash256) -> DangoResult<SearchTxOutcome> {
        let response = self.query_tx(hash).await?;
        let tx: Tx = response.tx.deserialize_json()?;

        Ok(SearchTxOutcome {
            tx,
            outcome: TxOutcome {
                gas_limit: response.tx_result.gas_wanted as u64,
                gas_used: response.tx_result.gas_used as u64,
                result: if response.tx_result.code == Code::Ok {
                    Ok(())
                } else {
                    Err(response.tx_result.log)
                },
                events: response.tx_result.data.deserialize_json()?,
            },
        })
    }

    async fn balance(&self, addr: Addr, denom: Denom) -> DangoResult<Uint128> {
        Ok(self.query_balance(addr, denom, None).await?.amount)
    }

    async fn contract_info(&self, addr: Addr) -> DangoResult<ContractInfo> {
        Ok(self.query_contract(addr, None).await?)
    }

    async fn query_wasm_smart<M, R>(
        &self,
        contract: Addr,
        msg: &M,
        height: Option<u64>,
    ) -> DangoResult<R>
    where
        M: Serialize + Send + Sync,
        R: DeserializeOwned,
    {
        Ok(self.query_wasm_smart(contract, msg, height).await?)
    }

    async fn send_message<S>(&self, signer: &mut S, msg: Message) -> DangoResult<Hash256>
    where
        S: Signer + Send + Sync,
    {
        let response = self
            .send_message(
                signer,
                msg,
                GasOption::Simulate {
                    scale: GAS_OPTION_SCALE,
                    flat_increase: GAS_OPTION_FLAT_INCREASE,
                },
            )
            .await?;

        Ok(response.hash.try_convert()?)
    }
}
