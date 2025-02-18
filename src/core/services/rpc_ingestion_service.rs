use crate::core::ports::blockchain::BlockchainPort;
use crate::core::ports::runner::Runner;
use crate::core::ports::storage::StoragePort;
use crate::messaging::{Event, Publisher};
use crate::utils::error::AggregatorResult;
use futures_util::StreamExt;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio_util::sync::CancellationToken;
use tracing::{error, info};

/// The `RpcIngestionService` is responsible for polling the blockchain for new transactions,
/// ingesting them, storing them, and publishing events.
pub struct RpcIngestionService {
    /// The blockchain adapter used to retrieve transactions.
    blockchain: Arc<dyn BlockchainPort>,
    /// The storage adapter used to store retrieved transactions.
    storage: Arc<dyn StoragePort>,
    /// The last processed slot number.
    last_processed_slot: AtomicU64,
    /// A cancellation token to gracefully stop the ingestion process.
    cancel_token: CancellationToken,
    /// An optional batch size for processing transactions.
    batch_size: Option<usize>,
    /// The maximum number of retry attempts for processing a slot.
    max_retry_attempts: usize,
    /// A publisher for emitting real-time events.
    publisher: Arc<Publisher>,
}

impl RpcIngestionService {
    /// Creates a new `RpcIngestionService` including a publisher.
    pub fn new(
        blockchain: Arc<dyn BlockchainPort>,
        storage: Arc<dyn StoragePort>,
        publisher: Arc<Publisher>,
        batch_size: Option<usize>,
        max_retry_attempts: usize,
        cancel_token: CancellationToken,
    ) -> Self {
        Self {
            blockchain,
            storage,
            last_processed_slot: AtomicU64::new(0),
            cancel_token,
            batch_size,
            max_retry_attempts,
            publisher,
        }
    }

    /// Sets the optional batch size for processing transactions.
    #[cfg(test)]
    pub fn with_batch_size(mut self, batch_size: Option<usize>) -> Self {
        self.batch_size = batch_size;
        self
    }

    #[cfg(test)]
    pub fn last_processed_slot(&self) -> &AtomicU64 {
        &self.last_processed_slot
    }
}

#[async_trait::async_trait]
impl Runner for RpcIngestionService {
    /// Runs the ingestion service.
    async fn run(&mut self) -> AggregatorResult<()> {
        let last_processed = self.last_processed_slot.load(Ordering::SeqCst);

        let slot_stream = self
            .blockchain
            .stream_current_epoch_transactions(
                self.cancel_token.clone(),
                last_processed,
                self.batch_size,
                self.max_retry_attempts,
            )
            .await?;
        futures_util::pin_mut!(slot_stream);

        // Use select to handle cancellation at the stream level
        while let Some(item) = tokio::select! {
            item = slot_stream.next() => item,
            _ = self.cancel_token.cancelled() => {
                info!("Processing cancelled. Shutting down gracefully.");
                None
            }
        } {
            match item {
                Ok((slot, txs)) => {
                    if !txs.is_empty() {
                        for tx in txs {
                            self.storage.store_transaction(tx.clone()).await?;
                            if let Err(e) = self.publisher.publish(Event::TransactionIngested {
                                transaction: tx.to_formatted(),
                            }) {
                                error!("Failed to publish TransactionIngested event: {}", e);
                                break;
                            }
                        }
                    }
                    self.last_processed_slot.store(slot, Ordering::SeqCst);
                }
                Err(e) => {
                    error!("Error processing slot batch: {}. Skipping slot.", e);
                    let _ = self.publisher.publish(Event::IngestionError {
                        error: e.to_string(),
                    });
                    continue;
                }
            }
        }

        Ok(())
    }
}
