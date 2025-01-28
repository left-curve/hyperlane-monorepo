use {
    crate::GrugProvider, async_trait::async_trait, hyperlane_core::{
        Announcement, ChainResult, HyperlaneChain, HyperlaneContract, HyperlaneDomain, HyperlaneProvider, SignedType, TxOutcome, ValidatorAnnounce, H256, U256
    },
};

/// A reference to a ValidatorAnnounce contract on Grug chain
#[derive(Debug)]
pub struct GrugValidatorAnnounce {
    domain: HyperlaneDomain,
    address: H256,
    provider: GrugProvider,
}

impl HyperlaneChain for GrugValidatorAnnounce {
    fn domain(&self) -> &HyperlaneDomain {
        &self.domain
    }

    fn provider(&self) -> Box<dyn HyperlaneProvider> {
        Box::new(self.provider.clone())
    }
}

impl HyperlaneContract for GrugValidatorAnnounce {
    fn address(&self) -> H256 {
        self.address
    }
}

#[async_trait]
impl ValidatorAnnounce for GrugValidatorAnnounce {
    /// Returns the announced storage locations for the provided validators.
    async fn get_announced_storage_locations(
        &self,
        validators: &[H256],
    ) -> ChainResult<Vec<Vec<String>>>{

        let payload = grug_hyperlane_types::va::QueryAnnounceStorageLocationsRequest{
            validators: validators.iter().map(|v| H160::from(v)).collect(),
        };

        let data = self.provider.query(&payload).await?;
    }

    /// Announce a storage location for a validator
    async fn announce(&self, announcement: SignedType<Announcement>) -> ChainResult<TxOutcome>{
        todo!()
    }

    /// Returns the number of additional tokens needed to pay for the announce
    /// transaction. Return `None` if the needed tokens cannot be determined.
    async fn announce_tokens_needed(&self, announcement: SignedType<Announcement>) -> Option<U256>{
        todo!()
    }
}
