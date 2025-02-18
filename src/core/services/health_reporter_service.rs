use crate::metrics::{HealthCheck, HealthChecker, HealthReport, HealthReporting};
use std::sync::Arc;

/// A service that aggregates health information from a Solana RPC adapter.
///
/// The `HealthReporterService` wraps a `HealthChecker` and implements both the
/// `HealthCheck` and `HealthReporting` traits. It can be used to query the health
/// of the Solana RPC adapter and generate a corresponding health report.
///
/// # Fields
///
/// * `solana_rpc_adapter` - A `HealthChecker` for the Solana RPC adapter. It is used to
///   determine whether the adapter is healthy.
pub struct HealthReporterService {
    solana_rpc_adapter: HealthChecker,
}

impl HealthReporterService {
    /// Creates a new `HealthReporterService`.
    ///
    /// This method takes a `HealthChecker` and returns an `Arc` wrapped instance of
    /// `HealthReporterService`. The `#[must_use]` attribute indicates that the returned
    /// value should not be discarded.
    ///
    /// # Arguments
    ///
    /// * `solana_rpc_adapter` - A `HealthChecker` to be used for health checking.
    ///
    /// # Returns
    ///
    /// An `Arc` containing the newly created `HealthReporterService`.
    #[must_use]
    pub fn new(solana_rpc_adapter: HealthChecker) -> Arc<Self> {
        Arc::new(Self { solana_rpc_adapter })
    }
}

impl HealthCheck for HealthReporterService {
    /// Checks whether the underlying Solana RPC adapter is healthy.
    ///
    /// This method delegates the health check to the inner `HealthChecker`.
    ///
    /// # Returns
    ///
    /// `true` if the Solana RPC adapter is healthy, or `false` otherwise.
    fn healthy(&self) -> bool {
        self.solana_rpc_adapter.healthy()
    }
}

impl HealthReporting for HealthReporterService {
    /// Generates a health report for the Solana RPC adapter.
    ///
    /// This method calls the inner health check multiple times (using `dbg!` for debugging)
    /// and returns a `HealthReport` indicating the health status of the Solana RPC adapter.
    ///
    /// The `#[must_use]` attribute indicates that the returned report should not be ignored.
    ///
    /// # Returns
    ///
    /// A `HealthReport` containing the health status of the Solana RPC adapter.
    #[must_use]
    fn report(&self) -> HealthReport {
        HealthReport {
            solana_rpc_adapter: self.solana_rpc_adapter.healthy(),
        }
    }
}
