use crate::adapters::api::server::ApiState;
use crate::adapters::TxStream;
use crate::messaging::{Event, Publisher};
use crate::utils::error::AggregatorError;
use actix::prelude::*;
use actix_web::{get, web, HttpRequest, HttpResponse};
use actix_web_actors::ws;
use futures::StreamExt;
use serde::Deserialize;
use serde_json;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::broadcast;
use tracing::info;
use web::Data;

/// A message used to send JSON strings over the WebSocket.
///
/// This message is sent internally by the actor to relay formatted JSON data to the client.
#[derive(Message)]
#[rtype(result = "()")]
pub struct WsEvent(String);

/// A WebSocket actor that manages sending stored transactions and live events to a client.
///
/// The actor uses two data sources:
/// - An initial stream of stored transactions (`initial_stream`), which is sent upon connection.
/// - A broadcast channel (`live_receiver`) for live events, sent continuously after stored data is done.
pub struct WsSubscription {
    /// A stream that yields stored transaction chunks.
    initial_stream: Option<Pin<TxStream>>,
    /// A broadcast receiver that listens for live events.
    live_receiver: broadcast::Receiver<Event>,
}

impl WsSubscription {
    /// Creates a new `WsSubscription` actor.
    ///
    /// # Arguments
    ///
    /// * `initial_stream` - A pinned stream providing chunks of stored transactions.
    /// * `live_receiver` - A broadcast receiver from which live events are obtained.
    ///
    /// # Returns
    ///
    /// A new instance of `WsSubscription`.
    pub fn new(initial_stream: Pin<TxStream>, live_receiver: broadcast::Receiver<Event>) -> Self {
        Self {
            initial_stream: Some(initial_stream),
            live_receiver,
        }
    }
}

impl Actor for WsSubscription {
    type Context = ws::WebsocketContext<Self>;

    /// Called when the actor starts.
    ///
    /// This method spawns two asynchronous tasks:
    /// 1. One that processes the stored transactions stream and sends each transaction as a JSON message.
    /// 2. One that listens for live events from the broadcast channel and sends them as JSON messages.
    fn started(&mut self, ctx: &mut Self::Context) {
        info!("WsSubscription started");
        let addr = ctx.address();

        if let Some(mut stream) = self.initial_stream.take() {
            actix_web::rt::spawn(async move {
                println!("Starting stored transactions stream");
                while let Some(chunk_result) = stream.next().await {
                    match chunk_result {
                        Ok(chunk) => {
                            println!("Sending stored chunk with {} transactions", chunk.len());
                            for tx in chunk {
                                // Convert transaction to formatted JSON.
                                let json = serde_json::to_string(&tx.to_formatted())
                                    .unwrap_or_else(|_| "{}".into());
                                addr.do_send(WsEvent(json));
                            }
                        }
                        Err(e) => {
                            eprintln!("Error in stored transactions stream: {}", e);
                        }
                    }
                }
                info!("Stored transactions stream finished, switching to live events");
                // addr.do_send(WsEvent("Switching to live updates.".into())); // Optional message but will be sent to the client
            });
        }

        let addr_live = ctx.address();
        let mut live_rx = self.live_receiver.resubscribe();
        actix_web::rt::spawn(async move {
            loop {
                match live_rx.recv().await {
                    Ok(event) => {
                        let json = serde_json::to_string(&event).unwrap_or_else(|_| "{}".into());
                        addr_live.do_send(WsEvent(json));
                    }
                    Err(e) => {
                        eprintln!("Error receiving live event: {}", e);
                        break;
                    }
                }
            }
        });
    }
}

impl Handler<WsEvent> for WsSubscription {
    type Result = ();

    /// Handles incoming `WsEvent` messages by sending the contained JSON text to the client.
    ///
    /// # Arguments
    ///
    /// * `msg` - The `WsEvent` message containing the JSON string.
    /// * `ctx` - The WebSocket context used to communicate with the client.
    fn handle(&mut self, msg: WsEvent, ctx: &mut Self::Context) {
        ctx.text(msg.0);
    }
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for WsSubscription {
    /// Processes incoming WebSocket messages.
    ///
    /// This implementation responds to pings with a pong, optionally handles text messages,
    /// and closes the connection when a close message is received.
    ///
    /// # Arguments
    ///
    /// * `msg` - The incoming WebSocket message (or protocol error).
    /// * `ctx` - The WebSocket context.
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        match msg {
            Ok(ws::Message::Ping(msg)) => ctx.pong(&msg),
            Ok(ws::Message::Pong(_)) => {}
            Ok(ws::Message::Text(_)) => { /* Optionally handle client messages */ }
            Ok(ws::Message::Close(reason)) => {
                ctx.close(reason);
                ctx.stop();
            }
            _ => (),
        }
    }
}

/// Parameters for the WebSocket subscription endpoint.
///
/// These query parameters allow the client to optionally specify a custom chunk size for
/// streaming stored transactions.
#[derive(Debug, Deserialize)]
pub struct SubscriptionParams {
    /// Optional chunk size for streaming stored transactions.
    pub chunk_size: Option<usize>,
}

/// HTTP handler for upgrading an HTTP request to a WebSocket connection for subscriptions.
///
/// This endpoint retrieves stored transactions (as a stream) from storage, subscribes to live events
/// from a publisher, and starts a `WsSubscription` actor that sends this data to the client over a WebSocket.
///
/// # Arguments
///
/// * `req` - The incoming HTTP request.
/// * `stream` - The payload stream of the HTTP request.
/// * `publisher` - Shared `Publisher` used to subscribe to live events.
/// * `params` - Optional query parameters that may include `chunk_size`.
/// * `state` - Shared API state containing the blockchain and storage adapters.
///
/// # Returns
///
/// A `Result<HttpResponse, AggregatorError>` where `Ok` indicates a successful upgrade to WebSocket,
/// and `Err` represents an error during the upgrade or actor initialization.
///
/// # Example
///
/// A client can connect to this endpoint at `/subscribe` (optionally specifying `chunk_size`):
/// ```
/// // Connect to: /subscribe?chunk_size=50
/// ```
#[get("/subscribe")]
pub async fn subscribe_ws(
    req: HttpRequest,
    stream: web::Payload,
    publisher: Data<Arc<Publisher>>,
    params: Option<web::Query<SubscriptionParams>>,
    state: Data<Arc<ApiState>>,
) -> Result<HttpResponse, AggregatorError> {
    // Determine the chunk size from the query parameter; default to 100 if not specified.
    let chunk_size = params.as_ref().and_then(|p| p.chunk_size).unwrap_or(100);
    // Retrieve a stream of stored transactions from storage, divided into chunks.
    let initial_stream = state
        .storage
        .stream_all_transactions_chunks(chunk_size)
        .await;
    // Subscribe to live events from the publisher.
    let receiver = publisher.subscribe();
    // Create the WebSocket subscription actor.
    let ws = WsSubscription::new(initial_stream, receiver);
    // Upgrade the HTTP connection to a WebSocket connection and start the actor.
    ws::start(ws, &req, stream).map_err(|e| AggregatorError::ApiError(e.to_string()))
}
