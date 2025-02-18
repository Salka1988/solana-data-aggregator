use crate::adapters::api::handlers::ws::subscribe_ws;
use crate::adapters::api::handlers::{
    accounts::get_account,
    health_check,
    transactions::{get_transaction, get_transactions_by_date},
};
use crate::core::ports::blockchain::BlockchainPort;
use crate::core::ports::storage::StoragePort;
use crate::messaging::Publisher;
use crate::metrics::{HealthCheck, HealthCheckReporting, HealthReporting};
use crate::utils::error::{AggregatorError, AggregatorResult};
use actix_web::dev::Service;
use actix_web::{dev::ServiceRequest, middleware, web, App, HttpResponse, HttpServer};
use futures_util::future::{ok, Either};
use std::env;
use std::net::Ipv4Addr;
use std::sync::Arc;

type HealthReporter = Arc<dyn HealthCheckReporting>;

/// Shared API state.
pub struct ApiState {
    pub blockchain: Arc<dyn BlockchainPort>,
    pub storage: Arc<dyn StoragePort>,
}

/// Launches the Actix‑web API server with logging, security, and endpoint registration.
///
/// This function creates and runs an HTTP server that listens on the specified host and port.
/// It registers several endpoints, including health checks, account retrieval, transaction
/// queries, and a WebSocket subscription endpoint. The server is secured using an API key
/// (provided via the `APP__SERVER__API_KEY` environment variable).
///
/// # Arguments
///
/// * `host` - The IP address on which the server should listen.
/// * `port` - The port on which the server should accept connections.
/// * `blockchain` - An Arc-wrapped reference to a blockchain adapter implementing `BlockchainPort`.
/// * `storage` - An Arc-wrapped reference to a storage adapter implementing `StoragePort`.
/// * `health_reporter` - An Arc-wrapped reference to a type implementing both `HealthCheck` and `HealthReporting`.
/// * `publisher` - An Arc-wrapped reference to a `Publisher` used for propagating events to WebSocket clients.
///
/// # Returns
///
/// An `AggregatorResult<()>` which is `Ok(())` if the server is started and runs successfully,
/// or an `AggregatorError` if any error occurs during the initialization or execution of the server.
///
/// # Environment Variables
///
/// The API key is read from the `APP__SERVER__API_KEY` environment variable. If it is not set,
/// the API key defaults to an empty string, and the server will reject requests lacking the correct key.
///
pub async fn launch_api_server<P>(
    host: Ipv4Addr,
    port: u16,
    blockchain: Arc<dyn BlockchainPort>,
    storage: Arc<dyn StoragePort>,
    health_reporter: Arc<P>,
    publisher: Arc<Publisher>,
) -> AggregatorResult<()>
where
    P: HealthCheck + HealthReporting + 'static,
{
    let api_key = Arc::new(env::var("APP__SERVER__API_KEY").unwrap_or_default());
    let state = Arc::new(ApiState {
        blockchain,
        storage,
    });

    HttpServer::new(move || {
        let api_key = api_key.clone();

        App::new()
            .wrap(middleware::Logger::default())
            .wrap(middleware::Logger::default()) // Logging middleware
            .wrap(middleware::NormalizePath::new(
                middleware::TrailingSlash::Trim,
            ))
            .wrap(middleware::DefaultHeaders::new().add(("X-Version", "1.0")))
            .wrap(middleware::Compress::default())
            .wrap_fn(move |req: ServiceRequest, srv| {
                if req.headers().get("X-API-KEY").and_then(|h| h.to_str().ok())
                    == Some(api_key.as_str())
                {
                    Either::Left(srv.call(req))
                } else {
                    let res = req.into_response(
                        HttpResponse::Forbidden()
                            .body("Forbidden: invalid API key")
                            .map_into_right_body(),
                    );
                    Either::Right(ok(res))
                }
            })
            .app_data(web::Data::new(state.clone()))
            .app_data(web::Data::new(health_reporter.clone() as HealthReporter))
            .app_data(web::Data::new(publisher.clone()))
            .service(health_check)
            .service(get_transaction)
            .service(get_transactions_by_date)
            .service(get_account)
            .service(subscribe_ws)
    })
    .bind((host, port))
    .map_err(|e| AggregatorError::ApiError(e.to_string()))?
    .run()
    .await
    .map_err(|e| AggregatorError::ApiError(e.to_string()))?;

    Ok(())
}
