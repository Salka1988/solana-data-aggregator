pub mod solana_client;
pub mod solana_stream;
pub mod transaction_parser;

#[cfg(test)]
mod tests {
    use crate::adapters::blockchain::solana_client::SolanaClientAdapter;
    use crate::core::ports::blockchain::BlockchainPort;
    use crate::utils::error::AggregatorError;
    use chrono::{TimeZone, Utc};
    use tokio;
    use tokio_util::sync::CancellationToken;

    #[test]
    fn test_parse_timestamp_valid() {
        let adapter = SolanaClientAdapter::new("http://dummy", 3);
        let block_time = 1_600_000_000; // Example timestamp.
        let result = adapter.parse_timestamp(block_time).unwrap();
        let expected = Utc.timestamp_opt(block_time, 0).unwrap();
        assert_eq!(result, expected);
    }

    #[tokio::test]
    async fn test_get_account_invalid_account_id() {
        let adapter = SolanaClientAdapter::new("http://dummy", 3);
        let result = adapter.get_account("invalid_account_id").await;
        assert!(result.is_err());
        if let Err(AggregatorError::ProcessingError(msg)) = result {
            assert!(msg.contains("Invalid account id:"));
        } else {
            panic!("Expected ProcessingError, got {:?}", result);
        }
    }

    #[tokio::test]
    async fn test_get_transaction_invalid_signature() {
        let adapter = SolanaClientAdapter::new("http://dummy", 3);
        let result = adapter.get_transaction("invalid_signature").await;
        assert!(result.is_err());
        if let Err(AggregatorError::InvalidTransaction(msg)) = result {
            assert!(msg.contains("Invalid signature format"));
        } else {
            panic!("Expected InvalidTransaction, got {:?}", result);
        }
    }

    #[tokio::test]
    async fn test_get_epoch_info_dummy() {
        let adapter = SolanaClientAdapter::new("http://dummy", 3);
        let result = adapter.get_epoch_info().await;
        assert!(result.is_err());
        if let Err(AggregatorError::BlockchainError(_)) = result {
        } else {
            panic!("Expected BlockchainError, got {:?}", result);
        }
    }

    #[tokio::test]
    async fn test_get_slot_dummy() {
        let adapter = SolanaClientAdapter::new("http://dummy", 3);
        let result = adapter.get_slot().await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_stream_current_epoch_transactions_dummy() {
        let adapter = SolanaClientAdapter::new("http://dummy", 3);
        let cancel_token = CancellationToken::new();
        let result = adapter
            .stream_current_epoch_transactions(cancel_token, 0, None, 3)
            .await;
        assert!(result.is_err());
    }
}
