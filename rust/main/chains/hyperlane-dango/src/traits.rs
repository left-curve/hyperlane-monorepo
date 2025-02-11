use {
    crate::{DangoError, DangoResult},
    grug::{EncodedBytes, Encoder, Hash256, Inner},
    hyperlane_core::{H160, H256, H512},
    tendermint::Hash as TmHash,
};

pub trait HashConvertor<T> {
    fn convert(self) -> T;
}

impl HashConvertor<H256> for Hash256 {
    fn convert(self) -> H256 {
        H256::from_slice(&self)
    }
}

impl HashConvertor<H512> for Hash256 {
    fn convert(self) -> H512 {
        let mut bytes = [0u8; 64];
        bytes[32..].copy_from_slice(&self);
        bytes.into()
    }
}

impl HashConvertor<Hash256> for H256 {
    fn convert(self) -> Hash256 {
        Hash256::from_inner(self.to_fixed_bytes())
    }
}

impl<E> HashConvertor<H256> for EncodedBytes<[u8; 20], E>
where
    E: Encoder,
{
    fn convert(self) -> H256 {
        let mut bytes = [0u8; 32];
        bytes[12..].copy_from_slice(&self);
        bytes.into()
    }
}

impl<E> HashConvertor<H160> for EncodedBytes<[u8; 20], E>
where
    E: Encoder,
{
    fn convert(self) -> H160 {
        self.inner().into()
    }
}

impl<E> HashConvertor<EncodedBytes<[u8; 20], E>> for H160
where
    E: Encoder,
{
    fn convert(self) -> EncodedBytes<[u8; 20], E> {
        EncodedBytes::from_inner(self.to_fixed_bytes())
    }
}

// ------------------------------ TryHashConvertor -----------------------------

pub trait TryHashConvertor<T> {
    fn try_convert(self) -> DangoResult<T>;
}

impl TryHashConvertor<Hash256> for H512 {
    fn try_convert(self) -> DangoResult<Hash256> {
        if self[..32] != [0; 32] {
            return Err(DangoError::conversion::<Hash256, _, _>(
                self,
                "first 32 bytes are not zero.",
            ));
        }

        let mut bytes = [0u8; 32];
        bytes.copy_from_slice(&self[32..]);
        Ok(bytes.into())
    }
}

impl<E> TryHashConvertor<EncodedBytes<[u8; 20], E>> for H256
where
    E: Encoder,
{
    fn try_convert(self) -> DangoResult<EncodedBytes<[u8; 20], E>> {
        if self[..12] != [0; 12] {
            return Err(DangoError::conversion::<
                EncodedBytes<[u8; 20], E>,
                _,
                _,
            >(self, "first 12 bytes are not zero."));
        }

        let mut bytes = [0u8; 20];
        bytes.copy_from_slice(&self[12..]);
        Ok(bytes.into())
    }
}

impl TryHashConvertor<Hash256> for TmHash {
    fn try_convert(self) -> DangoResult<Hash256> {
        match self {
            TmHash::Sha256(bytes) => Ok(Hash256::from_inner(bytes)),
            TmHash::None => Err(DangoError::conversion::<Hash256, _, _>(
                self,
                "hash is None.",
            )),
        }
    }
}
