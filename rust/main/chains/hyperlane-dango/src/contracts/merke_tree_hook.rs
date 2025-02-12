use {
    crate::{
        get_block_height_for_reorg_period, hyperlane_contract, provider::DangoProvider,
        DangoResult, HashConvertor, TryHashConvertor,
    },
    async_trait::async_trait,
    dango_hyperlane_types::{hooks::merkle, IncrementalMerkleTree as DangoIncrementalMerkleTree},
    hyperlane_core::{
        accumulator::incremental::IncrementalMerkle, ChainCommunicationError, ChainResult,
        Checkpoint, HyperlaneContract, HyperlaneDomain, MerkleTreeHook, ReorgPeriod, H256,
    },
};

#[derive(Debug)]
pub struct DangoMerkleTreeHook {
    address: H256,
    domain: HyperlaneDomain,
    provider: DangoProvider,
}

hyperlane_contract!(DangoMerkleTreeHook);

#[async_trait]
impl MerkleTreeHook for DangoMerkleTreeHook {
    async fn tree(&self, reorg_period: &ReorgPeriod) -> ChainResult<IncrementalMerkle> {
        let dango_tree = self.tree_raw(reorg_period).await?;

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
        Ok(self.tree_raw(reorg_period).await?.count as u32)
    }

    async fn latest_checkpoint(&self, reorg_period: &ReorgPeriod) -> ChainResult<Checkpoint> {
        let dango_tree = self.tree_raw(reorg_period).await?;

        Ok(Checkpoint {
            merkle_tree_hook_address: self.address(),
            mailbox_domain: self.domain.id(),
            root: dango_tree.root().convert(),
            index: dango_tree.count as u32,
        })
    }
}

impl DangoMerkleTreeHook {
    async fn tree_raw(
        &self,
        reorg_period: &ReorgPeriod,
    ) -> DangoResult<DangoIncrementalMerkleTree> {
        let block_height = get_block_height_for_reorg_period(&self.provider, reorg_period).await?;

        self.provider
            .query_wasm_smart(
                self.address.try_convert()?,
                merkle::QueryTreeRequest {},
                block_height,
            )
            .await
    }
}
