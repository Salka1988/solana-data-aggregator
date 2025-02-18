use crate::utils::error::AggregatorResult;
use async_trait::async_trait;

/// A trait representing a task or service that can be executed or run.
///
/// The `Runner` trait abstracts the execution of a long-running or background process.
/// Implementors of this trait should encapsulate any logic that needs to run continuously
/// or be triggered periodically. The trait is asynchronous and requires mutable access,
/// which is useful when the running process maintains state.
///
/// The trait is marked with `#[cfg_attr(feature = "test-helpers", mockall::automock)]` so that,
/// when enabled, a mock implementation is automatically generated for testing purposes.
#[async_trait]
#[cfg_attr(feature = "test-helpers", mockall::automock)]
pub trait Runner: Send + Sync {
    /// Runs the task or service.
    ///
    /// This asynchronous method is responsible for starting and executing the runner's
    /// logic. It may be used to start background tasks, polling operations, or any other
    /// long-running process. The method should return an `AggregatorResult<()>` indicating
    /// success or failure of the run operation.
    ///
    /// # Returns
    ///
    /// * `Ok(())` if the runner completes successfully.
    /// * An appropriate error variant wrapped in an `AggregatorResult` if an error occurs.
    async fn run(&mut self) -> AggregatorResult<()>;
}
