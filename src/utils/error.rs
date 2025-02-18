use crate::messaging::Event;
use actix_web::ResponseError;
use thiserror::Error;

/// Represents errors that can occur in the application.
///
/// The `AggregatorError` enum covers errors from various sources, such as blockchain operations,
/// storage operations, API logic, transaction parsing, configuration issues, processing errors,
/// and task join errors.
#[derive(Error, Debug)]
pub enum AggregatorError {
    #[error("Blockchain error: {0}")]
    BlockchainError(String),

    #[error("Storage error: {0}")]
    StorageError(String),

    #[error("API error: {0}")]
    ApiError(String),

    #[error("Invalid transaction: {0}")]
    InvalidTransaction(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("Processing error: {0}")]
    ProcessingError(String),

    #[error("Join error: {0}")]
    JoinError(#[from] tokio::task::JoinError),

    #[error("Event error: {0}")]
    EventError(String),
}

pub type AggregatorResult<T> = Result<T, AggregatorError>;

impl From<tokio::sync::broadcast::error::SendError<Event>> for AggregatorError {
    fn from(err: tokio::sync::broadcast::error::SendError<Event>) -> Self {
        AggregatorError::EventError(err.to_string())
    }
}

impl ResponseError for AggregatorError {}
