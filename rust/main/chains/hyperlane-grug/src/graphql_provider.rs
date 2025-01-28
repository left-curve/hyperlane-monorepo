use {
    async_trait::async_trait,
    hyperlane_core::{
        BlockInfo, ChainInfo, ChainResult, HyperlaneChain, HyperlaneDomain, HyperlaneProvider,
        TxnInfo, H256, H512, U256,
    },
};

/// Abstraction over a connection to a Grug chain
#[derive(Debug, Clone)]
pub struct GrugProvider {
    domain: HyperlaneDomain,
}

impl HyperlaneChain for GrugProvider {
    /// Return the domain
    fn domain(&self) -> &HyperlaneDomain {
        &self.domain
    }

    /// A provider for the chain
    fn provider(&self) -> Box<dyn HyperlaneProvider> {
        Box::new(self.clone())
    }
}

#[async_trait]
impl HyperlaneProvider for GrugProvider {
    /// Get block info for a given block height
    async fn get_block_by_height(&self, _height: u64) -> ChainResult<BlockInfo> {
        todo!()
    }

    /// Get txn info for a given txn hash
    async fn get_txn_by_hash(&self, _hash: &H512) -> ChainResult<TxnInfo> {
        todo!()
    }

    /// Returns whether a contract exists at the provided address
    async fn is_contract(&self, _address: &H256) -> ChainResult<bool> {
        todo!()
    }

    /// Fetch the balance of the wallet address associated with the chain provider.
    async fn get_balance(&self, _address: String) -> ChainResult<U256> {
        todo!()
    }

    /// Fetch metrics related to this chain
    async fn get_chain_metrics(&self) -> ChainResult<Option<ChainInfo>> {
        todo!()
    }
}
