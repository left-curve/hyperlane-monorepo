mod config;
mod log;
mod outcome;
use hyperlane_core::ReorgPeriod;
pub use {config::*, log::*, outcome::*};

pub enum ExecutionBlock {
    /// Default reorg period of Hyperlane
    ReorgPeriod(ReorgPeriod),
    /// Execute query at specific block height.
    Defined(u64),
}

impl Into<ExecutionBlock> for ReorgPeriod {
    fn into(self) -> ExecutionBlock {
        ExecutionBlock::ReorgPeriod(self)
    }
}

impl From<&ReorgPeriod> for ExecutionBlock {
    fn from(period: &ReorgPeriod) -> ExecutionBlock {
        ExecutionBlock::ReorgPeriod(period.clone())
    }
}
