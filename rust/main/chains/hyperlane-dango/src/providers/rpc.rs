use {
    super::DangoProviderInterface,
    crate::{
        BlockOutcome, BlockResultOutcome, DangoError, DangoResult, SearchTxOutcome,
        TryDangoConvertor,
    },
    async_trait::async_trait,
    grug::{
        Addr, ContractInfo, CronOutcome, Denom, GasOption, Hash256, JsonDeExt, Message, NonEmpty,
        Query, QueryRequest, Signer, SigningClient, Tx, TxOutcome, Uint128,
    },
    serde::{de::DeserializeOwned, Serialize},
    std::ops::Deref,
    tendermint::abci::{self, Code},
};

#[async_trait]
impl DangoProviderInterface for SigningClient {
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

    async fn get_block_result(&self, height: Option<u64>) -> DangoResult<BlockResultOutcome> {
        let response = self.query_block_result(height).await?;

        Ok(BlockResultOutcome {
            hash: Hash256::try_from(response.app_hash.as_bytes())?,
            height: response.height.value(),
            txs: response
                .txs_results
                .unwrap_or_default()
                .into_iter()
                .map(from_tm_tx_result)
                .collect::<DangoResult<_>>()?,
            cronjobs: response
                .finalize_block_events
                .into_iter()
                .map(from_tm_cron_result)
                .collect::<DangoResult<_>>()?,
        })
    }

    async fn search_tx(&self, hash: Hash256) -> DangoResult<SearchTxOutcome> {
        let response = self.query_tx(hash).await?;
        let tx: Tx = response.tx.deserialize_json()?;

        Ok(SearchTxOutcome::new(
            response.height.value(),
            tx,
            from_tm_tx_result(response.tx_result)?,
        ))
    }

    async fn balance(&self, addr: Addr, denom: Denom) -> DangoResult<Uint128> {
        Ok(self.query_balance(addr, denom, None).await?.amount)
    }

    async fn contract_info(&self, addr: Addr) -> DangoResult<ContractInfo> {
        Ok(self.query_contract(addr, None).await?)
    }

    async fn query_wasm_smart<R>(
        &self,
        contract: Addr,
        req: R,
        height: Option<u64>,
    ) -> DangoResult<R::Response>
    where
        R: QueryRequest + Send + Sync + 'static,
        R::Message: Serialize + Send + Sync + 'static,
        R::Response: DeserializeOwned,
    {
        Ok(self.deref().query_wasm_smart(contract, req, height).await?)
    }

    async fn query_app_config<T>(&self) -> DangoResult<T>
    where
        T: DeserializeOwned,
    {
        Ok(self
            .query_app(&Query::app_config(), None)
            .await?
            .as_app_config()
            .deserialize_json()?)
    }

    async fn broadcast_message<S>(
        &self,
        signer: &mut S,
        msg: Message,
        gas: GasOption,
    ) -> DangoResult<Hash256>
    where
        S: Signer + Send + Sync,
    {
        let response = self.send_message(signer, msg, gas).await?;

        Ok(response.hash.try_convert()?)
    }

    async fn simulate_message<S>(&self, signer: &S, msg: Message) -> DangoResult<TxOutcome>
    where
        S: Signer + Send + Sync,
    {
        let unsigned_tx = signer.unsigned_transaction(NonEmpty::new(vec![msg])?, &self.chain_id)?;
        Ok(self.simulate(&unsigned_tx).await?)
    }
}

fn from_tm_tx_result(tm_tx_result: abci::types::ExecTxResult) -> DangoResult<TxOutcome> {
    Ok(TxOutcome {
        gas_limit: tm_tx_result.gas_wanted as u64,
        gas_used: tm_tx_result.gas_used as u64,
        result: if tm_tx_result.code == Code::Ok {
            Ok(())
        } else {
            Err(tm_tx_result.log)
        },
        events: tm_tx_result.data.deserialize_json()?,
    })
}

fn from_tm_cron_result(tm_cron_result: abci::Event) -> DangoResult<CronOutcome> {
    Ok(CronOutcome {
        gas_limit: None,
        gas_used: 0,
        cron_event: tm_cron_result
            .attributes
            .first()
            .ok_or(DangoError::CronEvtNotFound {})?
            .value_str()?
            .deserialize_json()?,
    })
}
