use {
    crate::{provider::HyperlaneDangoProvider, DangoProvider, HashConvertor, TryHashConvertor},
    async_trait::async_trait,
    dango_hyperlane_types::va::{ExecuteMsg, QueryAnnouncedStorageLocationsRequest},
    grug::{Coins, HexByteArray, Message, TestAccount},
    hyperlane_core::{
        Announcement, ChainCommunicationError, ChainResult, HyperlaneChain, HyperlaneContract,
        HyperlaneDomain, HyperlaneProvider, SignedType, TxOutcome, ValidatorAnnounce, H256, U256,
    },
    std::{
        collections::{BTreeMap, BTreeSet},
        fmt::Debug,
        ops::DerefMut,
        sync::Arc,
    },
    tokio::sync::RwLock,
};

#[derive(Debug)]
pub struct DangoValidatorAnnounce<P>
where
    P: DangoProvider,
{
    provider: HyperlaneDangoProvider<P>,
    address: H256,
    signer: Arc<RwLock<TestAccount>>,
}

impl<P> HyperlaneContract for DangoValidatorAnnounce<P>
where
    P: DangoProvider + Clone + Debug + Send + Sync + 'static,
    ChainCommunicationError: From<P::Error>,
{
    fn address(&self) -> H256 {
        self.address
    }
}

impl<P> HyperlaneChain for DangoValidatorAnnounce<P>
where
    P: DangoProvider + Clone + Debug + Send + Sync + 'static,
    ChainCommunicationError: From<P::Error>,
{
    fn domain(&self) -> &HyperlaneDomain {
        self.provider.domain()
    }

    fn provider(&self) -> Box<dyn HyperlaneProvider> {
        self.provider.provider()
    }
}

#[async_trait]
impl<P> ValidatorAnnounce for DangoValidatorAnnounce<P>
where
    P: DangoProvider + Clone + Debug + Send + Sync + 'static,
    ChainCommunicationError: From<P::Error>,
{
    /// Returns the announced storage locations for the provided validators.
    async fn get_announced_storage_locations(
        &self,
        validators: &[H256],
    ) -> ChainResult<Vec<Vec<String>>> {
        let validators = validators
            .iter()
            .map(|v| v.try_convert())
            .collect::<ChainResult<BTreeSet<_>>>()?;

        let response = self
            .provider
            .query_wasm_smart::<_, BTreeMap<HexByteArray<20>, BTreeSet<String>>>(
                self.address.try_convert()?,
                &QueryAnnouncedStorageLocationsRequest { validators },
                None,
            )
            .await?;

        Ok(response
            .into_iter()
            .map(|(_, v)| v.into_iter().collect())
            .collect())
    }

    /// Announce a storage location for a validator
    async fn announce(&self, announcement: SignedType<Announcement>) -> ChainResult<TxOutcome> {
        let msg = ExecuteMsg::Announce {
            validator: announcement.value.validator.convert(),
            storage_location: announcement.value.storage_location,
            signature: HexByteArray::<65>::try_from(announcement.signature.to_vec()).map_err(
                |_| ChainCommunicationError::ParseError {
                    msg: "unable to parse signature".to_string(),
                },
            )?,
        };

        let msg = Message::execute(self.address.try_convert()?, &msg, Coins::new()).unwrap();

        let signer = self.signer.clone();

        let res = self
            .provider
            .send_message(signer.write().await.deref_mut(), msg)
            .await?;

        todo!()
    }
    /// Returns the number of additional tokens needed to pay for the announce
    /// transaction. Return `None` if the needed tokens cannot be determined.
    async fn announce_tokens_needed(&self, announcement: SignedType<Announcement>) -> Option<U256> {
        todo!()
    }
}
