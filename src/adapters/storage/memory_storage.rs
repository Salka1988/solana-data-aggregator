use crate::core::domain::transaction::Transaction;
use crate::core::ports::storage::StoragePort;
use crate::utils::error::{AggregatorError, AggregatorResult};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use futures_util::{stream, Stream, StreamExt};
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::RwLock;

/// An in-memory storage adapter for transactions.
///
/// This adapter implements the `StoragePort` trait using a thread-safe `HashMap` wrapped in an
/// asynchronous RwLock. Transactions are stored and retrieved by their signature (as a string).
///
/// # Note
///
/// This implementation is intended for temporary or testing purposes. In the future, support for
/// persistent storage using SQL/Postgres (or another database) is planned.
#[derive(Default, Clone)]
pub struct MemoryStorageAdapter {
    transactions: Arc<RwLock<HashMap<String, Transaction>>>,
}

impl MemoryStorageAdapter {
    /// Creates a new instance of `MemoryStorageAdapter`.
    ///
    /// # Examples
    ///
    /// ```
    /// use solana_data_aggregator::adapters::storage::memory_storage::MemoryStorageAdapter;
    /// let storage = MemoryStorageAdapter::new();
    /// ```
    pub fn new() -> Self {
        Self {
            transactions: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

#[async_trait]
impl StoragePort for MemoryStorageAdapter {
    /// Stores a transaction in the in-memory database.
    ///
    /// The transaction is inserted into the underlying HashMap using its signature (converted to a string)
    /// as the key.
    ///
    /// # Arguments
    ///
    /// * `transaction` - The transaction to store.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if the transaction is stored successfully, or an error otherwise.
    async fn store_transaction(&self, transaction: Transaction) -> AggregatorResult<()> {
        let mut map = self.transactions.write().await;
        map.insert(transaction.signature.to_string(), transaction);
        Ok(())
    }

    /// Retrieves a transaction from the in-memory storage by its signature.
    ///
    /// # Arguments
    ///
    /// * `signature` - A string slice representing the transaction signature.
    ///
    /// # Returns
    ///
    /// On success, returns the cloned `Transaction`. If no transaction with
    /// returns a `StorageError`.
    async fn get_transaction(&self, signature: &str) -> AggregatorResult<Transaction> {
        let map = self.transactions.read().await;
        map.get(signature)
            .cloned()
            .ok_or_else(|| AggregatorError::StorageError("Transaction not found".into()))
    }

    /// Retrieves transactions that occurred on a specific date.
    ///
    /// This method filters stored transactions based on their timestamp. Only transactions whose
    /// `timestamp` matches the provided date (ignoring time) are returned.
    ///
    /// # Arguments
    ///
    /// * `date` - A `DateTime<Utc>` representing the date for which transactions should be retrieved.
    ///
    /// # Returns
    ///
    /// Returns a vector of transactions that match the specified date. If no transactions match,
    /// an empty vector is returned.
    async fn get_transactions_by_date(
        &self,
        date: DateTime<Utc>,
    ) -> AggregatorResult<Vec<Transaction>> {
        let map = self.transactions.read().await;
        let result = map
            .values()
            .filter(|tx| tx.timestamp.date_naive() == date.date_naive())
            .cloned()
            .collect();
        Ok(result)
    }

    /// Streams all stored transactions in chunks.
    ///
    /// This method retrieves all transaction keys from the in-memory storage,
    /// divides them into chunks of the given size, and then processes each chunk
    /// asynchronously. For each chunk, it fetches the corresponding transactions and
    /// yields a stream item containing a vector of transactions.
    ///
    /// # Arguments
    ///
    /// * `chunk_size` - The maximum number of transactions to include in each chunk.
    ///
    /// # Returns
    ///
    /// A pinned stream that yields `AggregatorResult<Vec<Transaction>>` items. Each item is
    /// either a vector of transactions retrieved from the storage or an error if a transaction
    /// cannot be fetched.
    async fn stream_all_transactions_chunks(
        &self,
        chunk_size: usize,
    ) -> Pin<Box<dyn Stream<Item = AggregatorResult<Vec<Transaction>>> + Send + 'static>>
    where
        Self: Clone + 'static,
    {
        println!("Acquiring keys from storage...");
        let keys: Vec<String> = {
            let guard = self.transactions.read().await;
            guard.keys().cloned().collect()
        };
        println!("Found {} keys", keys.len());

        let chunks: Vec<Vec<String>> = keys
            .chunks(chunk_size)
            .map(|chunk| chunk.to_vec())
            .collect();
        println!("Divided into {} chunks", chunks.len());

        let adapter_clone = self.clone();
        let stream = stream::iter(chunks).then(move |chunk| {
            let adapter = adapter_clone.clone();
            async move {
                println!("Processing a chunk with {} keys", chunk.len());
                let mut transactions = Vec::new();
                for key in chunk {
                    let tx = adapter.get_transaction(&key).await?;
                    transactions.push(tx);
                }
                println!("Yielding chunk with {} transactions", transactions.len());
                Ok(transactions)
            }
        });
        Box::pin(stream)
    }
}
