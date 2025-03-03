use {
    bip32::{Language, Mnemonic},
    dango_client::SigningKey,
    dango_types::account_factory::Username,
    grug::{Addr, HexByteArray},
    hyperlane_base::settings::SignerConf,
    std::str::FromStr,
};

use super::constants::COIN_TYPE;

#[derive(Clone)]
pub struct UserInfo<'a> {
    pub username: Username,
    pub mnemonic: &'a str,
    pub address: Addr,
    pub sk: HexByteArray<32>,
}

impl<'a> UserInfo<'a> {
    pub fn new(username: &str, mnemonic: &'a str, address: &str) -> Self {
        let singing_key = SigningKey::from_mnemonic(
            &Mnemonic::new(mnemonic, Language::English).unwrap(),
            COIN_TYPE,
        )
        .unwrap();
        let sk = HexByteArray::from(singing_key.private_key());

        Self {
            username: Username::from_str(username).unwrap(),
            mnemonic,
            address: Addr::from_str(address).unwrap(),
            sk,
        }
    }
}

impl From<UserInfo<'_>> for SignerConf {
    fn from(value: UserInfo<'_>) -> Self {
        SignerConf::Dango {
            username: value.username,
            key: value.sk,
            address: value.address,
        }
    }
}
