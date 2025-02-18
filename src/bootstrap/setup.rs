use crate::adapters::blockchain::solana_client::SolanaClientAdapter;
use crate::adapters::health_tracking_adapter::HealthTrackingAdapter;
use crate::adapters::storage::memory_storage::MemoryStorageAdapter;
use crate::config::Config;
use crate::core::services::rpc_ingestion_service::RpcIngestionService;
use crate::messaging::event_listener::{run_event_listener, EventListener};
use crate::messaging::Publisher;
use crate::utils::scheduler::schedule_polling;
use std::sync::Arc;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

pub async fn setup_rpc_ingestion_service(
    config: &Config,
    solana_rpc: Arc<HealthTrackingAdapter<SolanaClientAdapter>>,
    storage_adapter: Arc<MemoryStorageAdapter>,
    publisher: Arc<Publisher>,
    cancel_token: CancellationToken,
) -> JoinHandle<()> {
    let rpc_ingestion_service = RpcIngestionService::new(
        solana_rpc,
        storage_adapter,
        publisher,
        config.blockchain.max_batch_size,
        config.blockchain.max_retry_attempts,
        cancel_token.clone(),
    );

    schedule_polling(
        config.polling.interval,
        rpc_ingestion_service,
        "RpcIngestionService",
        cancel_token,
    )
}

pub fn spawn_event_listener(
    publisher: Arc<Publisher>,
    cancel_token: CancellationToken,
    event_listener: Arc<EventListener>,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        run_event_listener(publisher, cancel_token, event_listener).await;
    })
}
