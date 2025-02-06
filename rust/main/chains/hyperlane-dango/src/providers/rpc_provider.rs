use {
    crate::HyperlaneDangoError,
    dango_client::{SigningKey, SingleSigner},
    grug::{
        Addr, Coin, Defined, Denom, Hash256, Inner, Message, NonEmpty, SigningClient, Undefined,
        __private::serde::{de::DeserializeOwned, Serialize},
    },
    hyperlane_core::{ChainResult, H256},
    std::{fmt::Debug, str::FromStr},
    tendermint_rpc::endpoint::{block, broadcast::tx_sync, tx},
    url::Url,
};

pub struct RpcProvider {
    client: SigningClient,
    sk: SigningKey,
    signer: SingleSigner<Defined<u32>>,
}

impl Debug for RpcProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RpcProvider")
            .field("client", &self.client)
            .field("sk", &self.sk)
            .field("signer username", &self.signer.username)
            .field("signer addr", &self.signer.address)
            .field("signer nonce", &self.signer.nonce)
            .finish()
    }
}

impl Clone for RpcProvider {
    // Cloning the provider will clone the client and the signer.
    // The signer should not be a problem since there will be only one instance of the signer
    // that sign transactions. The others are just for querying the chain.
    fn clone(&self) -> Self {
        Self {
            client: self.client.clone(),
            sk: self.sk.clone(),
            signer: SingleSigner::<Undefined<u32>>::new(
                self.signer.username.inner(),
                self.signer.address,
                self.sk.clone(),
            )
            .unwrap()
            .with_nonce(*self.signer.nonce.inner()),
        }
    }
}

impl RpcProvider {
    /// Create new `RpcProvider`
    pub async fn new(
        url: &Url,
        chain_id: &str,
        username: &str,
        address: &str,
        sk: SigningKey,
    ) -> ChainResult<Self> {
        let tendermint_url = tendermint_rpc::Url::try_from(url.to_owned())
            .map_err(Into::<HyperlaneDangoError>::into)?;
        let client = SigningClient::connect(chain_id, tendermint_url)
            .map_err(Into::<HyperlaneDangoError>::into)?;

        let signer = SingleSigner::<Undefined<u32>>::new(
            username,
            Addr::from_str(address).unwrap(),
            sk.clone(),
        )
        .unwrap();
        let signer = signer.query_nonce(&client).await.unwrap();
        return Ok(Self { client, sk, signer });
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
    pub async fn send_messages(
        &mut self,
        msgs: NonEmpty<Vec<Message>>,
    ) -> ChainResult<tx_sync::Response> {
        Ok(self
            .client
            .send_messages(
                &mut self.signer,
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
