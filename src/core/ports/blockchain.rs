use crate::core::domain::account::Account;
use crate::core::domain::transaction::Transaction;
use crate::utils::error::AggregatorResult;
use async_trait::async_trait;
use futures_util::Stream;
use solana_sdk::clock::Slot;
use solana_sdk::epoch_info::EpochInfo;
use std::pin::Pin;
use tokio_util::sync::CancellationToken;

/// A trait defining the blockchain operations required by the application.
///
/// This trait abstracts the operations needed to interact with the blockchain,
/// such as fetching transactions, streaming epoch transactions, retrieving epoch
/// information, fetching the current slot, and querying account information.
///
/// All methods are asynchronous and return an `AggregatorResult` that wraps either
/// the expected result or an error. This trait is intended to be implemented by types
/// that interface with blockchain nodes or services (e.g., RPC clients).
///
/// The trait is marked with `#[cfg_attr(feature = "test-helpers", mockall::automock)]`
/// so that when the `test-helpers` feature is enabled, a mock implementation is automatically generated.
#[async_trait]
#[cfg_attr(feature = "test-helpers", mockall::automock)]
pub trait BlockchainPort: Send + Sync {
    /// Retrieves a transaction from the blockchain using the provided signature.
    ///
    /// # Arguments
    ///
    /// * `signature` - A string slice representing the transaction's signature.
    ///
    /// # Returns
    ///
    /// An `AggregatorResult` containing a `Transaction` if successful, or an error otherwise.
    async fn get_transaction(&self, signature: &str) -> AggregatorResult<Transaction>;

    /// Streams transactions for the current epoch starting from a specified slot.
    ///
    /// This method creates a stream that yields transaction batches for each slot.
    /// The stream will yield items of type `AggregatorResult<(Slot, Vec<Transaction>)>`,
    /// where each tuple contains a slot number and the corresponding transactions.
    ///
    /// # Arguments
    ///
    /// * `cancel_token` - A `CancellationToken` that can be used to cancel the stream.
    /// * `last_processed_slot` - The last slot that was processed, so the stream can start
    ///   from the next slot.
    /// * `batch_size` - An optional parameter to specify the maximum number of transactions
    ///   per batch.
    ///
    /// # Returns
    ///
    /// An `AggregatorResult` containing a pinned, boxed stream that yields either a successful
    /// (Slot, Vec<Transaction>) tuple or an error.
    async fn stream_current_epoch_transactions(
        &self,
        cancel_token: CancellationToken,
        last_processed_slot: u64,
        batch_size: Option<usize>,
        max_retry_attempts: usize,
    ) -> AggregatorResult<
        Pin<Box<dyn Stream<Item = AggregatorResult<(Slot, Vec<Transaction>)>> + Send>>,
    >;

    /// Retrieves the current epoch information from the blockchain.
    ///
    /// # Returns
    ///
    /// An `AggregatorResult` containing an `EpochInfo` object on success, or an error otherwise.
    async fn get_epoch_info(&self) -> AggregatorResult<EpochInfo>;

    /// Retrieves the current slot number from the blockchain.
    ///
    /// # Returns
    ///
    /// An `AggregatorResult` containing the current slot (as a `Slot`) on success, or an error otherwise.
    async fn get_slot(&self) -> AggregatorResult<Slot>;

    /// Retrieves account information for a given account ID.
    ///
    /// # Arguments
    ///
    /// * `account_id` - A string slice representing the account's public key.
    ///
    /// # Returns
    ///
    /// An `AggregatorResult` containing an `Account` object if the account is found,
    /// or an error otherwise.
    async fn get_account(&self, account_id: &str) -> AggregatorResult<Account>;
}
