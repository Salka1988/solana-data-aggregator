pub mod event_listener;

use crate::core::domain::transaction::FormattedTransaction;
use crate::utils::error::AggregatorResult;
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

/// Represents events that can be emitted by the system.
///
/// These events are used to notify different parts of the application about important occurrences,
/// such as the ingestion of a new transaction or an error during ingestion.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Event {
    /// Emitted when a transaction has been successfully ingested.
    ///
    /// Contains a formatted representation of the transaction.
    TransactionIngested { transaction: FormattedTransaction },
    /// Emitted when an error occurs during the ingestion process.
    ///
    /// Contains a string with a description of the error.
    IngestionError { error: String },
}

/// A simple event publisher based on Tokio's broadcast channel.
///
/// The `Publisher` allows multiple subscribers to receive events concurrently. It is used
/// to propagate events (such as transaction ingestion or errors) throughout the application.
#[derive(Clone)]
pub struct Publisher {
    sender: broadcast::Sender<Event>,
}

impl Publisher {
    /// Creates a new `Publisher` with the specified buffer size.
    ///
    /// The buffer size determines how many events can be queued before older events are dropped.
    ///
    /// # Arguments
    ///
    /// * `buffer` - The capacity of the broadcast channel.
    ///
    pub fn new(buffer: usize) -> Self {
        let (sender, _) = broadcast::channel(buffer);
        Self { sender }
    }

    /// Publishes an event to all subscribers.
    ///
    /// The event is sent through the broadcast channel, and the function returns the number of
    /// subscribers that received the event. If an error occurs (for example, if there are no receivers),
    /// the error is converted into an `AggregatorError`.
    ///
    /// # Arguments
    ///
    /// * `event` - The event to publish.
    ///
    /// # Returns
    ///
    /// An `AggregatorResult<usize>` which contains the number of subscribers that received the event,
    /// or an error if the event could not be sent.
    pub fn publish(&self, event: Event) -> AggregatorResult<usize> {
        self.sender.send(event).map_err(|e| e.into())
    }

    /// Returns a new receiver subscribed to the event stream.
    ///
    /// Each receiver will get a copy of the events published after it subscribes.
    ///
    /// # Returns
    ///
    /// A `broadcast::Receiver<Event>` which can be used to receive published events.
    pub fn subscribe(&self) -> broadcast::Receiver<Event> {
        self.sender.subscribe()
    }
}
