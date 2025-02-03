use {
    grug::{Addr, HexByteArray},
    hyperlane_core::{ChainCommunicationError, ChainResult, H160, H256},
};

pub trait ToDangoAddr {
    fn to_dango_addr(&self) -> ChainResult<Addr>;
}

impl ToDangoAddr for H256 {
    fn to_dango_addr(&self) -> ChainResult<Addr> {
        Addr::try_from(&self.as_fixed_bytes()[12..]).map_err(|_| {
            ChainCommunicationError::ParseError {
                msg: "unable to parse address".to_string(),
            }
        })
    }
}

pub trait ToDangoHexByteArray {
    fn to_dango_hex_byte_array(&self) -> ChainResult<HexByteArray<20>>;
}

impl ToDangoHexByteArray for H160 {
    fn to_dango_hex_byte_array(&self) -> ChainResult<HexByteArray<20>> {
        Ok(HexByteArray::<20>::from_inner(*self.as_fixed_bytes()))
    }
}
