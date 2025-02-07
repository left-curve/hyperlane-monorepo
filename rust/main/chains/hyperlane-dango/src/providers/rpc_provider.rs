use {
    crate::HyperlaneDangoError,
    grug::{
        Addr, Coin, Denom, Hash256, Message, NonEmpty, Signer, SigningClient,
        __private::serde::{de::DeserializeOwned, Serialize},
    },
    hyperlane_core::{ChainResult, H256},
    std::fmt::Debug,
    tendermint_rpc::endpoint::{block, broadcast::tx_sync, tx},
    url::Url,
};

#[derive(Debug, Clone)]
pub struct RpcProvider {
    client: SigningClient,
}

impl RpcProvider {
    /// Create new `RpcProvider`
    pub async fn new(url: &Url, chain_id: &str) -> ChainResult<Self> {
        let tendermint_url = tendermint_rpc::Url::try_from(url.to_owned())
            .map_err(Into::<HyperlaneDangoError>::into)?;
        let client = SigningClient::connect(chain_id, tendermint_url)
            .map_err(Into::<HyperlaneDangoError>::into)?;

        return Ok(Self { client });
    }

    /// Request block by block height if height is provided, otherwise return the latest block.
    pub async fn get_block(&self, height: Option<u64>) -> ChainResult<block::Response> {
        Ok(self
            .client
            .query_block(height)
            .await
            .map_err(Into::<HyperlaneDangoError>::into)?)
    }

    // Get tx by hash.
    pub async fn get_tx(&self, tx_hash: H256) -> ChainResult<tx::Response> {
        Ok(self
            .client
            .query_tx(Hash256::from(*tx_hash.as_fixed_bytes()))
            .await
            .map_err(Into::<HyperlaneDangoError>::into)?)
    }

    /// Return whether a contract exists at the provided address.
    pub async fn is_contract(&self, address: Addr) -> ChainResult<bool> {
        match self.client.query_contract(address, None).await {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }

    /// Return the balance of an address for a specific coin.
    pub async fn get_balance(&self, address: Addr, denom: Denom) -> ChainResult<Coin> {
        Ok(self
            .client
            .query_balance(address, denom, None)
            .await
            .map_err(Into::<HyperlaneDangoError>::into)?)
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
        Ok(self
            .client
            .query_wasm_smart(contract, msg, height)
            .await
            .map_err(Into::<HyperlaneDangoError>::into)?)
    }

    /// Broadcast a transaction to the chain.
    pub async fn send_messages<S>(
        self,
        signer: &mut S,
        msgs: NonEmpty<Vec<Message>>,
    ) -> ChainResult<tx_sync::Response>
    where
        S: Signer,
    {
        Ok(self
            .client
            .send_messages(
                signer,
                msgs,
                grug::GasOption::Simulate {
                    scale: 1.2,
                    flat_increase: 0,
                },
            )
            .await
            .map_err(Into::<HyperlaneDangoError>::into)?)
    }
}
