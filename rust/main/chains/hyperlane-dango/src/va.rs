use {
    crate::{ToDangoAddr, ToDangoHexByteArray},
    async_trait::async_trait,
    dango_hyperlane_types::va::{ExecuteMsg, QueryAnnouncedStorageLocationsRequest},
    grug::{Coins, HexByteArray, Message, SigningClient, TestAccount},
    hyperlane_core::{
        Announcement, ChainCommunicationError, ChainResult, HyperlaneChain, HyperlaneContract,
        HyperlaneDomain, HyperlaneProvider, SignedType, TxOutcome, ValidatorAnnounce, H256, U256,
    },
    std::{
        collections::{BTreeMap, BTreeSet},
        ops::DerefMut,
        sync::Arc,
    },
    tokio::sync::RwLock,
};

#[derive(Debug)]
pub struct DangoValidatorAnnounce {
    provider: SigningClient,
    address: H256,
    signer: Arc<RwLock<TestAccount>>,
}

impl HyperlaneContract for DangoValidatorAnnounce {
    fn address(&self) -> H256 {
        self.address
    }
}

impl HyperlaneChain for DangoValidatorAnnounce {
    fn domain(&self) -> &HyperlaneDomain {
        todo!()
        // self.provider.domain()
    }

    fn provider(&self) -> Box<dyn HyperlaneProvider> {
        todo!()
        // self.provider.provider()
    }
}

#[async_trait]
impl ValidatorAnnounce for DangoValidatorAnnounce {
    /// Returns the announced storage locations for the provided validators.
    async fn get_announced_storage_locations(
        &self,
        validators: &[H256],
    ) -> ChainResult<Vec<Vec<String>>> {
        let validators = validators
            .iter()
            .map(|v| {
                HexByteArray::<20>::try_from(v.as_bytes()).map_err(|_| {
                    ChainCommunicationError::ParseError {
                        msg: "unable to parse address".to_string(),
                    }
                })
            })
            .collect::<Result<BTreeSet<_>, _>>()?;
        let msg = QueryAnnouncedStorageLocationsRequest { validators };
        let response = self
            .provider
            .query_wasm_smart::<_, BTreeMap<HexByteArray<20>, BTreeSet<String>>>(
                self.address.to_dango_addr()?,
                &msg,
                None,
            )
            .await
            .unwrap();

        Ok(response
            .into_iter()
            .map(|(_, v)| v.into_iter().collect())
            .collect())
    }

    /// Announce a storage location for a validator
    async fn announce(&self, announcement: SignedType<Announcement>) -> ChainResult<TxOutcome> {
        let msg = ExecuteMsg::Announce {
            validator: announcement.value.validator.to_dango_hex_byte_array()?,
            storage_location: announcement.value.storage_location,
            signature: HexByteArray::<65>::try_from(announcement.signature.to_vec()).map_err(
                |_| ChainCommunicationError::ParseError {
                    msg: "unable to parse signature".to_string(),
                },
            )?,
        };

        let msg = Message::execute(self.address.to_dango_addr()?, &msg, Coins::new()).unwrap();

        let signer = self.signer.clone();

        let res = self
            .provider
            .send_message(signer.write().await.deref_mut(), msg, todo!())
            .await
            .unwrap();

        todo!()
    }
    /// Returns the number of additional tokens needed to pay for the announce
    /// transaction. Return `None` if the needed tokens cannot be determined.
    async fn announce_tokens_needed(&self, announcement: SignedType<Announcement>) -> Option<U256> {
        todo!()
    }
}
