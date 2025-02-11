use {
    crate::{hyperlane_contract, provider::HyperlaneDangoProvider},
    async_trait::async_trait,
    hyperlane_core::{Mailbox, H256},
};

#[derive(Debug)]
pub struct DangoMailbox {
    provider: HyperlaneDangoProvider,
    address: H256,
}

hyperlane_contract!(DangoMailbox);

// #[async_trait]
// impl Mailbox for DangoMailbox {   
//         /// Gets the current leaf count of the merkle tree
//         ///
//         /// - `reorg_period` is how far behind the current block to query, if not specified
//         ///   it will query at the latest block.
//         async fn count(&self, reorg_period: &ReorgPeriod) -> ChainResult<u32>;
    
//         /// Fetch the status of a message
//         async fn delivered(&self, id: H256) -> ChainResult<bool>;
    
//         /// Fetch the current default interchain security module value
//         async fn default_ism(&self) -> ChainResult<H256>;
    
//         /// Get the latest checkpoint.
//         async fn recipient_ism(&self, recipient: H256) -> ChainResult<H256>;
    
//         /// Process a message with a proof against the provided signed checkpoint
//         async fn process(
//             &self,
//             message: &HyperlaneMessage,
//             metadata: &[u8],
//             tx_gas_limit: Option<U256>,
//         ) -> ChainResult<TxOutcome>;
    
//         /// Process a message with a proof against the provided signed checkpoint
//         async fn process_batch(
//             &self,
//             _messages: &[BatchItem<HyperlaneMessage>],
//         ) -> ChainResult<BatchResult> {
//             // Batching is not supported by default
//             Err(ChainCommunicationError::BatchingFailed)
//         }
    
//         /// Try process the given operations as a batch. Returns the outcome of the
//         /// batch (if one was submitted) and the operations that were not submitted.
//         async fn try_process_batch<'a>(
//             &self,
//             _ops: Vec<&'a QueueOperation>,
//         ) -> ChainResult<BatchResult> {
//             // Batching is not supported by default
//             Err(ChainCommunicationError::BatchingFailed)
//         }
    
//         /// Estimate transaction costs to process a message.
//         async fn process_estimate_costs(
//             &self,
//             message: &HyperlaneMessage,
//             metadata: &[u8],
//         ) -> ChainResult<TxCostEstimate>;
    
//         /// Get the calldata for a transaction to process a message with a proof
//         /// against the provided signed checkpoint
//         fn process_calldata(&self, message: &HyperlaneMessage, metadata: &[u8]) -> Vec<u8>;
// }
