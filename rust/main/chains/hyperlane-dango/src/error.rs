use hyperlane_core::ChainCommunicationError;

#[derive(Debug, thiserror::Error)]
pub enum HyperlaneDangoError {
    /// Tendermint RPC Error
    #[error(transparent)]
    TendermintError(#[from] tendermint_rpc::error::Error),

    #[error(transparent)]
    AnyhowError(#[from] anyhow::Error),
}

impl From<HyperlaneDangoError> for ChainCommunicationError {
    fn from(value: HyperlaneDangoError) -> Self {
        ChainCommunicationError::from_other(value)
    }
}
