use {
    super::RpcProvider,
    crate::{DangoConnectionConf, ToDangoAddr},
    async_trait::async_trait,
    dango_types::auth::Metadata,
    grug::{
        Addr, Inner, JsonDeExt, Message, NonEmpty, Signer, Tx,
        __private::serde::{de::DeserializeOwned, Serialize},
    },
    hyperlane_core::{
        h512_to_bytes, BlockInfo, ChainCommunicationError, ChainInfo, ChainResult, HyperlaneChain,
        HyperlaneDomain, HyperlaneProvider, HyperlaneProviderError, TxnInfo, H160, H256, H512,
        U256,
    },
    tendermint_rpc::endpoint::broadcast::tx_sync,
};

#[derive(Debug, Clone)]
pub struct DangoProvider {
    domain: HyperlaneDomain,
    connection_conf: DangoConnectionConf,
    provider: RpcProvider,
}

impl HyperlaneChain for DangoProvider {
    fn domain(&self) -> &HyperlaneDomain {
        &self.domain
    }

    fn provider(&self) -> Box<dyn HyperlaneProvider> {
        Box::new(self.clone())
    }
}

#[async_trait]
impl HyperlaneProvider for DangoProvider {
    /// Get block info for a given block height
    async fn get_block_by_height(&self, height: u64) -> ChainResult<BlockInfo> {
        let response = self.provider.get_block(Some(height)).await?;

        let block = response.block;
        let block_height = block.header.height.value();

        if block_height != height {
            Err(HyperlaneProviderError::IncorrectBlockByHeight(
                height,
                block_height,
            ))?
        }

        let hash = H256::from_slice(response.block_id.hash.as_bytes());
        let time = block.header.time;

        Ok(BlockInfo {
            hash: hash.to_owned(),
            timestamp: time.unix_timestamp() as u64,
            number: block_height,
        })
    }

    /// Get txn info for a given txn hash
    async fn get_txn_by_hash(&self, hash: &H512) -> ChainResult<TxnInfo> {
        let hash = H256::from_slice(&h512_to_bytes(hash));
        let response = self.provider.get_tx(hash).await?;

        let response_hash = H256::from_slice(response.hash.as_bytes());

        if hash != response_hash {
            return Err(ChainCommunicationError::from_other_str(&format!(
                "received incorrect transaction, expected hash: {:?}, received hash: {:?}",
                hash, response_hash,
            )));
        }

        let tx = response.tx.deserialize_json::<Tx>().map_err(|err| {
            ChainCommunicationError::from_other_str(&format!(
                "failed to deserialize transaction with hash {:?}: {:?}",
                hash, err
            ))
        })?;

        let data = tx.data.deserialize_json::<Metadata>().map_err(|err| {
            ChainCommunicationError::from_other_str(&format!(
                "failed to deserialize data with hash {:?}: {:?}",
                hash, err
            ))
        })?;

        // let gas_price = self.calculate_gas_price(&hash, &tx)?;
        Ok(TxnInfo {
            hash: hash.into(),
            gas_limit: tx.gas_limit.into(),
            max_priority_fee_per_gas: None,
            max_fee_per_gas: None,
            gas_price: None, // TODO is this needed?
            nonce: data.nonce.into(),
            sender: H256::from_slice(tx.sender.inner()),
            recipient: self.recipient(tx.msgs),
            receipt: None,
            raw_input_data: None,
        })
    }

    /// Returns whether a contract exists at the provided address
    async fn is_contract(&self, address: &H256) -> ChainResult<bool> {
        let address = address.to_dango_addr()?;

        self.provider.is_contract(address).await
    }

    /// Fetch the balance of the wallet address associated with the chain provider.
    async fn get_balance(&self, address: String) -> ChainResult<U256> {
        let address =
            Addr::try_from(address.as_bytes()).map_err(|_| ChainCommunicationError::ParseError {
                msg: format!("failed to parse address {:?} into Addr", address),
            });

        let balance = self
            .provider
            .get_balance(address?, self.connection_conf.get_canonical_asset())
            .await?;

        Ok(U256::from_big_endian(&balance.amount.inner().to_be_bytes()))
    }

    /// Fetch metrics related to this chain
    async fn get_chain_metrics(&self) -> ChainResult<Option<ChainInfo>> {
        let block = self.provider.get_block(None).await?;
        return Ok(Some(ChainInfo {
            latest_block: BlockInfo {
                hash: H256::from_slice(block.block_id.hash.as_bytes()),
                timestamp: block.block.header.time.unix_timestamp() as u64,
                number: block.block.header.height.value(),
            },
            min_gas_price: None,
        }));
    }
}

impl DangoProvider {
    /// Assume there are exactly one Msg.
    pub fn recipient(&self, msgs: NonEmpty<Vec<Message>>) -> Option<H256> {
        let msg = msgs.first().unwrap();
        match msg {
            Message::Execute(msg) => Some(H160::from_slice(msg.contract.inner()).into()),
            Message::Transfer(msg) => Some(H160::from_slice(msg.to.inner()).into()), // TODO: is this needed?
            _ => None,
        }
    }

    /// Query a contract on the chain.
    pub async fn query_wasm_smart<M, R>(
        &self,
        contract: Addr,
        msg: &M,
        height: Option<u64>,
    ) -> ChainResult<R>
    where
        M: Serialize,
        R: DeserializeOwned,
    {
        self.provider.query_wasm_smart(contract, msg, height).await
    }

    pub async fn send_message<S>(
        &self,
        signer: &mut S,
        msg: Message,
    ) -> ChainResult<tx_sync::Response>
    where
        S: Signer,
    {
        self.provider.send_message(signer, msg).await
    }

    pub async fn send_messages<S>(
        &self,
        signer: &mut S,
        msgs: Vec<Message>,
    ) -> ChainResult<tx_sync::Response>
    where
        S: Signer,
    {
        let msgs = NonEmpty::new(msgs).unwrap();
        self.provider.send_messages(signer, msgs).await
    }
}
