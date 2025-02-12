use {
    crate::{provider::DangoProvider, DangoResult},
    hyperlane_core::ReorgPeriod,
};

/// Given a `reorg_period`, returns the block height at the moment.
/// If the `reorg_period` is None, a block height of None is given.
pub(crate) async fn get_block_height_for_reorg_period(
    provider: &DangoProvider,
    reorg_period: &ReorgPeriod,
) -> DangoResult<Option<u64>> {
    let block_height = match reorg_period {
        ReorgPeriod::Blocks(blocks) => {
            let last_block = provider.get_block(None).await?;
            let block_height = last_block.height - blocks.get() as u64;
            Some(block_height)
        }
        ReorgPeriod::None => None,
        ReorgPeriod::Tag(_) => {
            return Err(anyhow::anyhow!(
                "Tag reorg period is not supported in Dango MerkleTreeHook"
            )
            .into())
        }
    };

    Ok(block_height)
}
