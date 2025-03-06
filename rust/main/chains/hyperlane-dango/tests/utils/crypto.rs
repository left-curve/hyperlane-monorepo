use {
    ethers::signers::{LocalWallet, Signer},
    grug::HexByteArray,
    hyperlane_core::H256,
};

pub struct ValidatorKey {
    pub key: H256,
    pub address: HexByteArray<20>,
}

impl ValidatorKey {
    pub fn new_random() -> Self {
        let key = H256::random();
        let address = derive_address(&key);
        Self {
            key,
            address: HexByteArray::from_inner(address),
        }
    }
}

fn derive_address(key: &H256) -> [u8; 20] {
    let wallet = LocalWallet::from(ethers::core::k256::ecdsa::SigningKey::from(
        ethers::core::k256::SecretKey::from_be_bytes(key.as_bytes()).unwrap(),
    ));

    wallet.address().to_fixed_bytes()
}

pub fn derive_pk(key: &H256) -> [u8; 33] {
    ethers::core::k256::ecdsa::SigningKey::from(
        ethers::core::k256::SecretKey::from_be_bytes(key.as_bytes()).unwrap(),
    )
    .verifying_key()
    .to_bytes()
    .into()
}
