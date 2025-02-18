mod setup;

use crate::adapters::api::server::launch_api_server;
use crate::adapters::blockchain::solana_client::SolanaClientAdapter;
use crate::adapters::health_tracking_adapter::HealthTrackingAdapter;
use crate::adapters::storage::memory_storage::MemoryStorageAdapter;
use crate::config::Config;
use crate::core::services::health_reporter_service::HealthReporterService;
use crate::messaging::event_listener::EventListener;
use crate::messaging::Publisher;
use crate::utils::error::AggregatorResult;
use crate::utils::logger;
use clap::Parser;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use tracing::error;

#[derive(Parser, Debug)]
#[command(author, version, about)]
pub struct BootstrapArgs {
    #[arg(short, long)]
    pub config: Option<PathBuf>,
}

pub async fn run_bootstrap() -> AggregatorResult<()> {
    logger::init();

    let cancel_token = CancellationToken::new();
    let args = BootstrapArgs::parse();

    let config = match Config::new(args.config) {
        Ok(cfg) => cfg,
        Err(e) => {
            error!("Config error: {}", e);
            std::process::exit(1);
        }
    };
    config.validate()?;

    let solana_adapter = Arc::new(SolanaClientAdapter::new(
        &config.blockchain.rpc_url,
        config.polling.max_retry_attempts,
    ));
    let health_tracking_solana_adapter = Arc::new(HealthTrackingAdapter::new(
        solana_adapter.clone(),
        config.polling.max_retry_attempts,
    ));

    let storage_adapter = Arc::new(MemoryStorageAdapter::new());

    let mut process_handles = vec![];

    let publisher = Arc::new(Publisher::new(200));

    let rpc_ingestion_service = setup::setup_rpc_ingestion_service(
        &config,
        health_tracking_solana_adapter.clone(),
        storage_adapter.clone(),
        publisher.clone(),
        cancel_token.clone(),
    )
    .await;

    let event_listener = Arc::new(EventListener::new());
    let event_listener_handle = setup::spawn_event_listener(
        publisher.clone(),
        cancel_token.clone(),
        event_listener.clone(),
    );
    process_handles.push(event_listener_handle);

    let health_reporter =
        HealthReporterService::new(health_tracking_solana_adapter.connection_health_checker());

    process_handles.push(rpc_ingestion_service);

    launch_api_server(
        config.server.host,
        config.server.port,
        health_tracking_solana_adapter,
        storage_adapter,
        health_reporter,
        publisher,
    )
    .await?;

    shut_down(cancel_token, process_handles).await
}

pub async fn shut_down(
    cancel_token: CancellationToken,
    handles: Vec<JoinHandle<()>>,
) -> AggregatorResult<()> {
    cancel_token.cancel();

    for handle in handles {
        handle.await?;
    }

    Ok(())
}
