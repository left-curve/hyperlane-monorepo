use {
    crate::HyperlaneDangoError,
    grug::{Addr, Client, Coin, Denom, Hash256},
    hyperlane_core::{ChainCommunicationError, ChainResult, H256},
    std::fmt::Debug,
    tendermint_rpc::endpoint::{block, tx},
};

#[derive(Debug, Clone)]
pub struct RpcProvider {
    client: Client,
}

impl RpcProvider {
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
    pub async fn is_contract(&self, address: &H256) -> ChainResult<bool> {
        let address = Addr::try_from(&address.as_fixed_bytes()[12..]).map_err(|_| {
            ChainCommunicationError::ParseError {
                msg: "unable to parse address".to_string(),
            }
        })?;

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
}
