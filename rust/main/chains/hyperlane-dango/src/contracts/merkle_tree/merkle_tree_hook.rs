use {
    crate::{
        hyperlane_contract, provider::DangoProvider, ConnectionConf, DangoResult, DangoSigner,
        HashConvertor, TryHashConvertor,
    },
    async_trait::async_trait,
    dango_hyperlane_types::{hooks::merkle, IncrementalMerkleTree as DangoIncrementalMerkleTree},
    hyperlane_core::{
        accumulator::incremental::IncrementalMerkle, ChainCommunicationError, ChainResult,
        Checkpoint, ContractLocator, HyperlaneContract, MerkleTreeHook,
        ReorgPeriod, H256,
    },
};

#[derive(Debug)]
pub struct DangoMerkleTreeHook {
    address: H256,
    provider: DangoProvider,
}

hyperlane_contract!(DangoMerkleTreeHook);

#[async_trait]
impl MerkleTreeHook for DangoMerkleTreeHook {
    async fn tree(&self, reorg_period: &ReorgPeriod) -> ChainResult<IncrementalMerkle> {
        let dango_tree = self.dango_tree(reorg_period).await?;

        Ok(IncrementalMerkle::new(
            dango_tree
                .branch
                .into_iter()
                .map(|hash| hash.convert())
                .collect::<Vec<H256>>()
                .try_into()
                .map_err(|_| ChainCommunicationError::ParseError {
                    msg: "Failed to build merkle branch array".to_string(),
                })?,
            dango_tree.count as usize,
        ))
    }

    async fn count(&self, reorg_period: &ReorgPeriod) -> ChainResult<u32> {
        Ok(self.dango_tree(reorg_period).await?.count as u32)
    }

    async fn latest_checkpoint(&self, reorg_period: &ReorgPeriod) -> ChainResult<Checkpoint> {
        let dango_tree = self.dango_tree(reorg_period).await?;

        Ok(Checkpoint {
            merkle_tree_hook_address: self.address(),
            mailbox_domain: self.provider.domain.id(),
            root: dango_tree.root().convert(),
            index: dango_tree.count as u32,
        })
    }
}

impl DangoMerkleTreeHook {
    pub fn new(
        config: &ConnectionConf,
        locator: &ContractLocator,
        signer: Option<DangoSigner>,
    ) -> DangoResult<Self> {
        Ok(Self {
            provider: DangoProvider::from_config(config, locator.domain.clone(), signer)?,
            address: locator.address,
        })
    }

    /// Query the chain and return the DangoTree (same as IncrementalMerkleTree
    /// but with different values types).
    async fn dango_tree(
        &self,
        reorg_period: &ReorgPeriod,
    ) -> DangoResult<DangoIncrementalMerkleTree> {
        let block_height = self
            .provider
            .get_block_height_for_reorg_period(reorg_period)
            .await?;

        self.provider
            .query_wasm_smart(
                self.address.try_convert()?,
                merkle::QueryTreeRequest {},
                block_height,
            )
            .await
    }
}
