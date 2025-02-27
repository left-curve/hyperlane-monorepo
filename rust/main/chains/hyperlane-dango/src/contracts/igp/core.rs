use {
    crate::{ConnectionConf, DangoProvider, DangoResult},
    async_trait::async_trait,
    hyperlane_core::{ChainResult, HyperlaneDomain, InterchainGasPayment, SequenceAwareIndexer},
};

#[derive(Debug)]
pub struct IGP {
    pub provider: DangoProvider,
}

impl IGP {
    pub fn new(config: &ConnectionConf, domain: HyperlaneDomain) -> DangoResult<Self> {
        Ok(Self {
            provider: DangoProvider::from_config(config, domain, None)?,
        })
    }
}

#[async_trait]
impl SequenceAwareIndexer<InterchainGasPayment> for IGP {
    async fn latest_sequence_count_and_tip(&self) -> ChainResult<(Option<u32>, u32)> {
        let height = self.provider.get_block(None).await?.height;
        Ok((None, height as u32))
    }
}
