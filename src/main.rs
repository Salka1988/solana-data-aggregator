use solana_data_aggregator::bootstrap::run_bootstrap;
use solana_data_aggregator::utils::error::AggregatorResult;

#[tokio::main]
async fn main() -> AggregatorResult<()> {
    run_bootstrap().await
}
