use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

/// Initializes the global tracing subscriber for logging.
///
/// This function configures a tracing subscriber with:
/// - An environment filter that attempts to read the log level from the environment.
///   If that fails, it defaults to `"info"`.
/// - A formatting layer that prints log messages in a human-readable format.
///
/// Once initialized, all log messages (using the `tracing` crate) will be emitted according to this configuration.
///
pub fn init() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();
}
