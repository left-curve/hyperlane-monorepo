use std::fmt::Debug;

use grug::Hash256;
use hyperlane_core::ChainCommunicationError;

pub type DangoResult<T> = Result<T, DangoError>;

#[derive(Debug, thiserror::Error)]
pub enum DangoError {
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

    #[error("transaction not found: {hash}")]
    TxNotFound { hash: Hash256 },
}

impl DangoError {
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

impl From<DangoError> for ChainCommunicationError {
    fn from(value: DangoError) -> Self {
        ChainCommunicationError::from_other(value)
    }
}

pub trait IntoDangoError<T> {
    fn into_dango_error(self) -> Result<T, DangoError>;
}

impl<T, E> IntoDangoError<T> for Result<T, E>
where
    DangoError: From<E>,
{
    fn into_dango_error(self) -> Result<T, DangoError> {
        self.map_err(DangoError::from)
    }
}
