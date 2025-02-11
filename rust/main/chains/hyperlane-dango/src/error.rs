use std::fmt::Debug;

use hyperlane_core::ChainCommunicationError;

pub type HyperlaneDangoResult<T> = Result<T, HyperlaneDangoError>;

#[derive(Debug, thiserror::Error)]
pub enum HyperlaneDangoError {
    /// Tendermint RPC Error
    #[error(transparent)]
    TendermintError(#[from] tendermint_rpc::error::Error),

    #[error(transparent)]
    AnyhowError(#[from] anyhow::Error),

    #[error(transparent)]
    StdError(#[from] grug::StdError),

    #[error("failed to convert {ty_from} to {ty_to}: from {from}, reason: {reason}")]
    ConversionError {
        ty_from: &'static str,
        ty_to: &'static str,
        from: String,
        reason: String,
    },

    
}

impl HyperlaneDangoError {
    pub fn conversion<T, F, R>(from: F, reason: R) -> Self
    where
        F: Debug,
        R: ToString,
    {
        Self::ConversionError {
            ty_from: std::any::type_name::<F>(),
            ty_to: std::any::type_name::<T>(),
            from: format!("{:?}", from),
            reason: reason.to_string(),
        }
    }
}

impl From<HyperlaneDangoError> for ChainCommunicationError {
    fn from(value: HyperlaneDangoError) -> Self {
        ChainCommunicationError::from_other(value)
    }
}

pub trait IntoHyperlaneDangoError<T> {
    fn into_dango_error(self) -> Result<T, HyperlaneDangoError>;
}

impl<T, E> IntoHyperlaneDangoError<T> for Result<T, E>
where
    HyperlaneDangoError: From<E>,
{
    fn into_dango_error(self) -> Result<T, HyperlaneDangoError> {
        self.map_err(HyperlaneDangoError::from)
    }
}
