use {
    crate::{
        hyperlane_contract, provider::DangoProvider, ConnectionConf, DangoResult,
        DangoSigner, HashConvertor, IntoDangoError, TryHashConvertor,
    },
    async_trait::async_trait,
    dango_hyperlane_types::va::{ExecuteMsg, QueryAnnouncedStorageLocationsRequest},
    grug::{Coins, Message},
    hyperlane_core::{
        Announcement, ChainResult, ContractLocator, SignedType, TxOutcome, ValidatorAnnounce, H256,
        U256,
    },
    std::collections::BTreeSet,
};

#[derive(Debug)]
pub struct DangoValidatorAnnounce {
    provider: DangoProvider,
    address: H256,
}

impl DangoValidatorAnnounce {
    pub fn new(
        config: &ConnectionConf,
        locator: &ContractLocator,
        signer: Option<DangoSigner>,
    ) -> DangoResult<Self> {
        Ok(Self {
            provider: DangoProvider::from_config(config, locator.domain.clone(), signer)?,
            address: locator.address,
        })
    }
}

hyperlane_contract!(DangoValidatorAnnounce);

#[async_trait]
impl ValidatorAnnounce for DangoValidatorAnnounce {
    /// Returns the announced storage locations for the provided validators.
    async fn get_announced_storage_locations(
        &self,
        validators: &[H256],
    ) -> ChainResult<Vec<Vec<String>>> {
        let validators = validators
            .iter()
            .map(|v| v.try_convert())
            .collect::<DangoResult<BTreeSet<_>>>()?;

        let response = self
            .provider
            .query_wasm_smart(
                self.address.try_convert()?,
                QueryAnnouncedStorageLocationsRequest { validators },
                None,
            )
            .await?;

        Ok(response
            .into_values()
            .map(|v| v.into_iter().collect())
            .collect())
    }

    /// Announce a storage location for a validator
    async fn announce(&self, announcement: SignedType<Announcement>) -> ChainResult<TxOutcome> {
        let msg = ExecuteMsg::Announce {
            validator: announcement.value.validator.convert(),
            storage_location: announcement.value.storage_location,
            signature: announcement
                .signature
                .to_vec()
                .try_into()
                .into_dango_error()?,
        };

        let msg = Message::execute(self.address.try_convert()?, &msg, Coins::new()).unwrap();

        Ok(self.provider.send_message_and_find(msg, None).await?)
    }
    /// Returns the number of additional tokens needed to pay for the announce
    /// transaction. Return `None` if the needed tokens cannot be determined.
    async fn announce_tokens_needed(
        &self,
        _announcement: SignedType<Announcement>,
    ) -> Option<U256> {
        // TODO: Right now Dango has gas price = 0 so it doesn't need any tokens.
        // If in the future the gas price will be > 0, we should simulate the tx.
        Some(U256::zero())
    }
}
