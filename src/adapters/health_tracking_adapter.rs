use rand::rng;
use rand::Rng;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;
use tokio_util::sync::CancellationToken;
use tracing::warn;

use crate::core::domain::{account::Account, transaction::Transaction};
use crate::core::ports::blockchain::BlockchainPort;
use crate::utils::error::{AggregatorError, AggregatorResult};

use crate::metrics::connection_health_tracker::ConnectionHealthTracker;
use crate::metrics::{HealthCheck, HealthChecker};
use solana_sdk::{clock::Slot, epoch_info::EpochInfo};

/// A health tracking adapter that wraps an inner adapter (implementing `BlockchainPort`)
/// and monitors the connection health using a `ConnectionHealthTracker`.
///
/// This adapter intercepts calls to the underlying adapter for certain methods
/// (such as streaming transactions, getting epoch info, and retrieving the current slot)
/// and executes them with automatic retries based on the health tracking logic.
/// For other methods (like getting a transaction or account), it forwards the calls directly.
#[derive(Clone)]
pub struct HealthTrackingAdapter<T: ?Sized> {
    adapter: Arc<T>,
    health_tracker: Arc<ConnectionHealthTracker>,
}

impl<T: ?Sized> HealthTrackingAdapter<T> {
    /// Creates a new `HealthTrackingAdapter` wrapping the provided adapter.
    ///
    /// # Arguments
    ///
    /// * `adapter` - An `Arc` to an instance that implements `BlockchainPort`.
    /// * `unhealthy_after_n_errors` - The maximum number of consecutive errors allowed before
    ///   marking the service as unhealthy.
    pub fn new(adapter: Arc<T>, unhealthy_after_n_errors: usize) -> Self {
        Self {
            adapter,
            health_tracker: Arc::new(ConnectionHealthTracker::new(unhealthy_after_n_errors)),
        }
    }

    /// Returns a health checker that can be used to inspect the current health of the connection.
    #[must_use]
    pub fn connection_health_checker(&self) -> HealthChecker {
        self.health_tracker.tracker()
    }

    /// Checks whether the underlying service is currently considered healthy.
    pub fn is_healthy(&self) -> bool {
        self.health_tracker.healthy()
    }

    /// Executes an operation with automatic retries based on health tracking.
    ///
    /// If the adapter is not healthy, returns an error immediately.
    /// Otherwise, it will attempt the operation up to a maximum number of attempts, with exponential
    /// backoff (and jitter) between retries. The health tracker is updated after each attempt.
    ///
    /// # Arguments
    ///
    /// * `operation` - A closure that returns a future resolving to an `AggregatorResult<R>`.
    /// * `op_name` - A name for the operation, used for logging purposes.
    ///
    /// # Returns
    ///
    /// Returns `Ok(R)` if the operation eventually succeeds, or an `AggregatorError` if all attempts fail.
    pub async fn execute_with_health_retries<F, Fut, R>(
        &self,
        operation: F,
        op_name: &str,
    ) -> AggregatorResult<R>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = AggregatorResult<R>> + Send,
    {
        if !self.is_healthy() {
            return Err(AggregatorError::ApiError("Service is unhealthy".into()));
        }

        let max_attempts = self.health_tracker.max_consecutive_failures();
        let base_delay = 1; // Base delay in seconds.
        let mut last_error: Option<AggregatorError> = None;

        for attempt in 1..=max_attempts {
            let result = operation().await;

            match &result {
                Ok(_) => self.health_tracker.note_success(),
                Err(_) => self.health_tracker.note_failure(),
            }

            match result {
                Ok(value) => return Ok(value),
                Err(e) => {
                    last_error = Some(e);
                    warn!(
                        "Health tracker: Failed to execute '{}' on attempt {}/{} with error: {}",
                        op_name,
                        attempt,
                        max_attempts,
                        last_error.as_ref().unwrap()
                    );
                }
            }

            let jitter: f64 = rng().random_range(0.5..1.5);
            let delay_secs =
                ((base_delay * 2_u32.pow((attempt - 1) as u32)) as f64 * jitter).min(60.0);
            sleep(Duration::from_secs_f64(delay_secs)).await;
        }

        Err(AggregatorError::ApiError(format!(
            "{} failed after {} attempts: {}",
            op_name,
            max_attempts,
            last_error.unwrap()
        )))
    }
}

#[async_trait::async_trait]
impl<T: BlockchainPort + Send + Sync> BlockchainPort for HealthTrackingAdapter<T> {
    async fn get_transaction(&self, signature: &str) -> AggregatorResult<Transaction> {
        self.adapter.get_transaction(signature).await
    }

    async fn stream_current_epoch_transactions(
        &self,
        cancel_token: CancellationToken,
        last_processed_slot: u64,
        batch_size: Option<usize>,
        max_retry_attempts: usize,
    ) -> AggregatorResult<
        std::pin::Pin<
            Box<dyn futures_util::Stream<Item = AggregatorResult<(Slot, Vec<Transaction>)>> + Send>,
        >,
    > {
        self.execute_with_health_retries(
            move || {
                let ct = cancel_token.clone();
                self.adapter.stream_current_epoch_transactions(
                    ct,
                    last_processed_slot,
                    batch_size,
                    max_retry_attempts,
                )
            },
            "stream_current_epoch_transactions",
        )
        .await
    }

    async fn get_epoch_info(&self) -> AggregatorResult<EpochInfo> {
        self.execute_with_health_retries(|| self.adapter.get_epoch_info(), "get_epoch_info")
            .await
    }

    async fn get_slot(&self) -> AggregatorResult<Slot> {
        self.execute_with_health_retries(|| self.adapter.get_slot(), "get_slot")
            .await
    }

    async fn get_account(&self, account_id: &str) -> AggregatorResult<Account> {
        self.adapter.get_account(account_id).await
    }
}
