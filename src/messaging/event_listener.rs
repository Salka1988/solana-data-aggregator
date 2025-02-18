use crate::messaging::{Event, Publisher};
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::select;
use tokio::sync::broadcast::Receiver;
use tokio_util::sync::CancellationToken;
use tracing::{error, info, warn};

/// Statistics tracking for the event listener.
/// Keeps count of processed transactions and encountered errors.
#[derive(Default)]
struct EventStats {
    /// Counter for successfully processed transactions
    transactions_processed: AtomicUsize,
    /// Counter for errors encountered during processing
    errors_encountered: AtomicUsize,
}

/// Event listener for monitoring and processing system events.
/// Provides functionality to track and report on transaction processing
/// and error handling statistics.
#[derive(Default)]
pub struct EventListener {
    /// Atomic statistics counters wrapped in Arc for thread-safe access
    stats: Arc<EventStats>,
}

impl EventListener {
    /// Creates a new EventListener instance with initialized statistics.
    ///
    /// # Returns
    /// A new EventListener with zeroed statistics counters.
    pub fn new() -> Self {
        Self {
            stats: Arc::new(EventStats::default()),
        }
    }

    /// Retrieves the current processing statistics.
    ///
    /// # Returns
    /// A tuple containing:
    /// - Number of processed transactions
    /// - Number of encountered errors
    pub fn get_stats(&self) -> (usize, usize) {
        (
            self.stats.transactions_processed.load(Ordering::Relaxed),
            self.stats.errors_encountered.load(Ordering::Relaxed),
        )
    }
}

/// Runs the event listener loop, processing incoming events and managing statistics.
///
/// This function sets up a continuous event processing loop that handles transaction
/// events and errors, while maintaining statistics and implementing error rate limiting.
///
/// # Arguments
/// * `publisher` - Arc-wrapped Publisher instance for receiving events
/// * `cancel_token` - Token for graceful shutdown coordination
/// * `listener` - Arc-wrapped EventListener for statistics tracking
///
/// # Error Handling
/// - Implements error rate limiting to prevent log spam
/// - Handles broadcast channel lagging by resubscribing
/// - Tracks error statistics
///
/// # Future Improvements Planned
/// - Metrics integration (e.g., Prometheus)
/// - Recovery procedures implementation
/// - Health metrics system
pub async fn run_event_listener(
    publisher: Arc<Publisher>,
    cancel_token: CancellationToken,
    listener: Arc<EventListener>,
) {
    let mut receiver: Receiver<Event> = publisher.subscribe();
    let mut last_error_time = HashMap::new();
    let error_threshold = Duration::from_secs(300);

    info!("Event listener started");

    loop {
        select! {
            result = receiver.recv() => {
                match result {
                    Ok(event) => {
                        match event {
                            Event::TransactionIngested { .. } => {
                                listener.stats.transactions_processed.fetch_add(1, Ordering::Relaxed);
                                 // can find good use for this. At the moment its spamming the logs
                            }
                            Event::IngestionError { error } => {
                                listener.stats.errors_encountered.fetch_add(1, Ordering::Relaxed);

                                let now = Instant::now();
                                let should_log = last_error_time
                                    .get(&error)
                                    .map_or(true, |&last_time| now.duration_since(last_time) > error_threshold);

                                if should_log {
                                    error!("Ingestion error occurred: {}", error);
                                    last_error_time.insert(error.clone(), now);
                                }
                                //todo: add metrics
                                // maybe implement run()
                                // maybe prometheus or whatever
                                // need to add recovery stuff
                                // reconnects etc
                                // health metrics would be good too
                            }
                        }
                    }
                    Err(e) => {
                        warn!("Error receiving event: {}", e);
                        if e.to_string().contains("lagged") {
                            receiver = publisher.subscribe();
                            continue;
                        }
                        break;
                    }
                }
            }
            _ = cancel_token.cancelled() => {
                info!("Event listener cancelled gracefully");
                break;
            }
        }
    }

    let (tx_count, error_count) = listener.get_stats();
    info!(
        "Event listener shutting down. Processed {} transactions, encountered {} errors",
        tx_count, error_count
    );
}
