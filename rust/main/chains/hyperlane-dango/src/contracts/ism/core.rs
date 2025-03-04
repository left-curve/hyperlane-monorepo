use {
    crate::{
        hyperlane_contract, ConnectionConf, DangoConvertor, DangoProvider, DangoResult,
        DangoSigner, TryDangoConvertor,
    },
    async_trait::async_trait,
    dango_hyperlane_types::isms,
    hyperlane_core::{
        ChainResult, ContractLocator, HyperlaneMessage, InterchainSecurityModule, ModuleType,
        MultisigIsm, RawHyperlaneMessage, H256, U256,
    },
};

#[derive(Debug)]
pub struct DangoIsm {
    provider: DangoProvider,
    address: H256,
}

impl DangoIsm {
    pub fn new(
        config: &ConnectionConf,
        locator: &ContractLocator,
        signer: Option<DangoSigner>,
    ) -> DangoResult<Self> {
        Ok(Self {
            address: locator.address,
            provider: DangoProvider::from_config(config, locator.domain.clone(), signer)?,
        })
    }
}

hyperlane_contract!(DangoIsm);

#[async_trait]
impl InterchainSecurityModule for DangoIsm {
    async fn module_type(&self) -> ChainResult<ModuleType> {
        Ok(ModuleType::MessageIdMultisig)
    }

    async fn dry_run_verify(
        &self,
        message: &HyperlaneMessage,
        metadata: &[u8],
    ) -> ChainResult<Option<U256>> {
        self.provider
            .query_wasm_smart(
                self.address.try_convert()?,
                isms::multisig::QueryIsmRequest(isms::IsmQuery::Verify {
                    raw_message: RawHyperlaneMessage::from(message).into(),
                    raw_metadata: metadata.to_vec().into(),
                }),
                None,
            )
            .await?;

        // We don't have a way to estimate gas for this call, so we return a default value
        Ok(Some(U256::one()))
    }
}

#[async_trait]
impl MultisigIsm for DangoIsm {
    async fn validators_and_threshold(
        &self,
        message: &HyperlaneMessage,
    ) -> ChainResult<(Vec<H256>, u8)> {
        let res = self
            .provider
            .query_wasm_smart(
                self.address.try_convert()?,
                isms::multisig::QueryValidatorSetRequest {
                    domain: message.origin,
                },
                None,
            )
            .await?;

        let validators: Vec<H256> = res
            .validators
            .into_iter()
            .map(DangoConvertor::convert)
            .collect();

        Ok((validators, res.threshold as u8))
    }
}
