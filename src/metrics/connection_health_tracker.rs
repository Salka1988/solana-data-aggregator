use crate::metrics::{HealthCheck, HealthChecker};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

/// A tracker that monitors the health of a connection based on consecutive failures.
///
/// The `ConnectionHealthTracker` maintains an atomic counter of consecutive failures and
/// determines whether a connection is healthy by comparing this counter to a configured maximum.
/// If the number of consecutive failures is below the threshold, the connection is considered healthy.
#[derive(Debug, Clone)]
pub struct ConnectionHealthTracker {
    /// The maximum number of consecutive failures allowed before the connection is considered unhealthy.
    max_consecutive_failures: usize,
    /// An atomic counter tracking the current number of consecutive failures.
    consecutive_failures: Arc<AtomicUsize>,
}

impl HealthCheck for ConnectionHealthTracker {
    /// Checks if the connection is healthy.
    ///
    /// A connection is deemed healthy if the number of consecutive failures is less than the
    /// configured maximum. Otherwise, it is considered unhealthy.
    ///
    /// # Returns
    ///
    /// * `true` if the number of consecutive failures is less than `max_consecutive_failures`.
    /// * `false` otherwise.
    fn healthy(&self) -> bool {
        self.consecutive_failures.load(Ordering::Relaxed) < self.max_consecutive_failures
    }
}

impl ConnectionHealthTracker {
    /// Creates a new `ConnectionHealthTracker`.
    ///
    /// # Arguments
    ///
    /// * `max_consecutive_failures` - The maximum number of consecutive failures allowed before the
    ///   connection is considered unhealthy.
    ///
    /// # Returns
    ///
    /// A new instance of `ConnectionHealthTracker` with the failure counter initialized to zero.
    #[must_use]
    pub fn new(max_consecutive_failures: usize) -> Self {
        Self {
            max_consecutive_failures,
            consecutive_failures: Arc::new(AtomicUsize::new(0)),
        }
    }

    /// Records a failure by incrementing the consecutive failures counter.
    pub fn note_failure(&self) {
        self.consecutive_failures.fetch_add(1, Ordering::SeqCst);
    }

    /// Resets the consecutive failures counter to zero.
    pub fn note_success(&self) {
        self.consecutive_failures.store(0, Ordering::SeqCst);
    }

    /// Returns a boxed clone of this tracker as a `HealthChecker`.
    ///
    /// This is useful for obtaining a trait object that implements the `HealthCheck` trait,
    /// allowing the tracker to be used in contexts where a `HealthChecker` is required.
    ///
    /// # Returns
    ///
    /// A `HealthChecker` trait object wrapping a clone of this tracker.
    #[must_use]
    pub fn tracker(&self) -> HealthChecker {
        Box::new(self.clone())
    }

    /// Returns the maximum number of consecutive failures allowed.
    ///
    /// # Returns
    ///
    /// The maximum number of consecutive failures that are permitted before the connection is considered unhealthy.
    pub fn max_consecutive_failures(&self) -> usize {
        self.max_consecutive_failures
    }
}
