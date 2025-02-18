use crate::core::domain::transaction::Transaction;
use crate::utils::error::AggregatorResult;
use actix::dev::Stream;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use std::pin::Pin;

/// A trait defining the storage operations for transactions.
///
/// The `StoragePort` trait abstracts the storage backend used to store and retrieve
/// transactions. It is designed to be implemented by in-memory storage adapters,
/// as well as future persistent storage implementations (e.g. SQL/Postgres).
///
/// Each method is asynchronous and returns an `AggregatorResult` which either
/// contains the expected value or an error indicating why the operation failed.
///
/// The trait is annotated with `#[cfg_attr(feature = "test-helpers", mockall::automock)]`
/// to allow automatic generation of mock implementations for testing purposes.
#[async_trait]
#[cfg_attr(feature = "test-helpers", mockall::automock)]
pub trait StoragePort: Send + Sync {
    /// Stores a transaction in the storage backend.
    ///
    /// # Arguments
    ///
    /// * `transaction` - The transaction to be stored.
    ///
    /// # Returns
    ///
    /// Returns an `AggregatorResult<()>` indicating success or failure.
    async fn store_transaction(&self, transaction: Transaction) -> AggregatorResult<()>;

    /// Retrieves a transaction from storage using its signature.
    ///
    /// # Arguments
    ///
    /// * `signature` - A string slice representing the transaction signature.
    ///
    /// # Returns
    ///
    /// On success, returns the corresponding `Transaction`. If no transaction is found,
    /// an appropriate error is returned.
    async fn get_transaction(&self, signature: &str) -> AggregatorResult<Transaction>;

    /// Retrieves transactions that occurred on a specific date.
    ///
    /// This method filters stored transactions by comparing the date portion of their timestamps
    /// with the provided `date` parameter.
    ///
    /// # Arguments
    ///
    /// * `date` - A `DateTime<Utc>` value representing the date for which transactions should be retrieved.
    ///
    /// # Returns
    ///
    /// Returns an `AggregatorResult` containing a vector of `Transaction` objects that match the date.
    async fn get_transactions_by_date(
        &self,
        date: DateTime<Utc>,
    ) -> AggregatorResult<Vec<Transaction>>;

    async fn stream_all_transactions_chunks(
        &self,
        chunk_size: usize,
    ) -> Pin<Box<dyn Stream<Item = AggregatorResult<Vec<Transaction>>> + Send>>;
}
