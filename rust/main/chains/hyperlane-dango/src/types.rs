mod config;
mod convertor;
mod error;
mod log;
mod outcome;
mod signer;

pub use {config::*, convertor::*, error::*, log::*, outcome::*, signer::*};

pub enum ExecutionBlock {
    /// Default reorg period of Hyperlane
    ReorgPeriod(hyperlane_core::ReorgPeriod),
    /// Execute query at specific block height.
    Defined(u64),
}

impl From<hyperlane_core::ReorgPeriod> for ExecutionBlock {
    fn from(period: hyperlane_core::ReorgPeriod) -> ExecutionBlock {
        ExecutionBlock::ReorgPeriod(period)
    }
}
