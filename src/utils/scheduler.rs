use crate::core::ports::runner::Runner;
use std::time::Duration;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use tracing::{error, info};

/// Schedules a periodic polling task.
///
/// This function spawns an asynchronous task that repeatedly executes the provided `runner`'s `run()` method
/// at the specified polling interval. It logs any errors encountered during execution. The loop will break
/// if the provided cancellation token is triggered, at which point a stop message is logged.
///
/// # Arguments
///
/// * `polling_interval` - The duration to wait between consecutive runs of the runner.
/// * `runner` - An instance implementing the `Runner` trait whose `run()` method will be called periodically.
/// * `name` - A static string identifier used for logging purposes.
/// * `cancel_token` - A `CancellationToken` to allow graceful cancellation of the polling loop.
///
/// # Returns
///
/// Returns a `JoinHandle<()>` representing the spawned task.
pub fn schedule_polling(
    polling_interval: Duration,
    mut runner: impl Runner + 'static,
    name: &'static str,
    cancel_token: CancellationToken,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        loop {
            if let Err(e) = runner.run().await {
                error!("{name} encountered an error: {e}");
            }

            if cancel_token.is_cancelled() {
                break;
            }

            tokio::time::sleep(polling_interval).await;
        }

        info!("{name} stopped");
    })
}
