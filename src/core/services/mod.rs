pub mod health_reporter_service;
pub mod rpc_ingestion_service;

#[cfg(test)]
mod tests {
    use crate::core::domain::account::Account;
    use crate::core::domain::transaction::{Transaction, TransactionStatus};
    use crate::core::ports::blockchain::BlockchainPort;
    use crate::core::ports::runner::Runner;
    use crate::core::ports::storage::StoragePort;
    use crate::core::services::rpc_ingestion_service::RpcIngestionService;
    use crate::messaging::Publisher;
    use crate::utils::error::{AggregatorError, AggregatorResult};
    use async_trait::async_trait;
    use futures_util::{stream, Stream};
    use solana_sdk::{clock::Slot, pubkey::Pubkey, signature::Signature};
    use std::collections::HashMap;
    use std::pin::Pin;
    use std::sync::atomic::Ordering;
    use std::sync::{Arc, Mutex};
    use tokio_util::sync::CancellationToken;

    fn dummy_transaction() -> Transaction {
        let signature = Signature::from([1; 64]);
        let timestamp = chrono::Utc::now();
        let sender = Pubkey::new_unique();
        let receiver = Pubkey::new_unique();
        Transaction::new(
            signature,
            timestamp,
            sender,
            receiver,
            100,
            TransactionStatus::Success,
        )
    }

    #[derive(Clone)]
    struct DummyBlockchain;

    #[async_trait]
    impl BlockchainPort for DummyBlockchain {
        async fn get_transaction(&self, _signature: &str) -> AggregatorResult<Transaction> {
            Err(AggregatorError::ApiError("not implemented".into()))
        }

        async fn stream_current_epoch_transactions(
            &self,
            _cancel_token: CancellationToken,
            last_processed_slot: u64,
            _batch_size: Option<usize>,
            _max_retry_attempts: usize,
        ) -> AggregatorResult<
            Pin<Box<dyn Stream<Item = AggregatorResult<(Slot, Vec<Transaction>)>> + Send>>,
        > {
            let items: Vec<AggregatorResult<(Slot, Vec<Transaction>)>> = vec![
                Ok((last_processed_slot + 1, vec![dummy_transaction()])),
                Ok((last_processed_slot + 2, vec![])),
            ];
            Ok(Box::pin(stream::iter(items)))
        }

        async fn get_epoch_info(&self) -> AggregatorResult<solana_sdk::epoch_info::EpochInfo> {
            Err(AggregatorError::ApiError("not implemented".into()))
        }

        async fn get_slot(&self) -> AggregatorResult<Slot> {
            Err(AggregatorError::ApiError("not implemented".into()))
        }

        async fn get_account(&self, _account_id: &str) -> AggregatorResult<Account> {
            Err(AggregatorError::ApiError("not implemented".into()))
        }
    }

    /// A dummy storage adapter for testing, using an in-memory Mutex-based HashMap.
    #[derive(Clone)]
    struct DummyStorage {
        pub transactions: Arc<Mutex<HashMap<String, Transaction>>>,
    }

    impl DummyStorage {
        fn new() -> Self {
            Self {
                transactions: Arc::new(Mutex::new(HashMap::new())),
            }
        }
    }

    #[async_trait]
    impl StoragePort for DummyStorage {
        async fn store_transaction(&self, transaction: Transaction) -> AggregatorResult<()> {
            let mut map = self.transactions.lock().unwrap();
            map.insert(transaction.signature.to_string(), transaction);
            Ok(())
        }

        async fn get_transaction(&self, signature: &str) -> AggregatorResult<Transaction> {
            let map = self.transactions.lock().unwrap();
            map.get(signature)
                .cloned()
                .ok_or_else(|| AggregatorError::StorageError("Transaction not found".into()))
        }

        async fn get_transactions_by_date(
            &self,
            _date: chrono::DateTime<chrono::Utc>,
        ) -> AggregatorResult<Vec<Transaction>> {
            unimplemented!()
        }

        async fn stream_all_transactions_chunks(
            &self,
            _chunk_size: usize,
        ) -> Pin<Box<dyn Stream<Item = AggregatorResult<Vec<Transaction>>> + Send>> {
            unimplemented!()
        }
    }

    #[tokio::test]
    async fn test_rpc_ingestion_service_runs_successfully() {
        let dummy_blockchain = Arc::new(DummyBlockchain);
        let dummy_storage = Arc::new(DummyStorage::new());
        let cancel_token = CancellationToken::new();
        let publisher = Arc::new(Publisher::new(100));
        let mut service = RpcIngestionService::new(
            dummy_blockchain,
            dummy_storage.clone(),
            publisher,
            Some(1),
            3,
            cancel_token,
        );

        let result = service.run().await;
        assert!(result.is_ok());

        assert_eq!(service.last_processed_slot().load(Ordering::SeqCst), 2);

        let dummy_tx = dummy_transaction();
        let stored_tx = dummy_storage
            .get_transaction(&dummy_tx.signature.to_string())
            .await;
        assert!(stored_tx.is_ok());
    }

    #[tokio::test]
    async fn test_rpc_ingestion_service_handles_stream_error() {
        struct ErrorBlockchain;

        #[async_trait]
        impl BlockchainPort for ErrorBlockchain {
            async fn get_transaction(&self, _signature: &str) -> AggregatorResult<Transaction> {
                unimplemented!()
            }
            async fn stream_current_epoch_transactions(
                &self,
                _cancel_token: CancellationToken,
                _last_processed_slot: u64,
                _batch_size: Option<usize>,
                _max_retry_attempts: usize,
            ) -> AggregatorResult<
                Pin<Box<dyn Stream<Item = AggregatorResult<(Slot, Vec<Transaction>)>> + Send>>,
            > {
                let items: Vec<AggregatorResult<(Slot, Vec<Transaction>)>> =
                    vec![Err(AggregatorError::ApiError("stream error".into()))];
                Ok(Box::pin(stream::iter(items)))
            }
            async fn get_epoch_info(&self) -> AggregatorResult<solana_sdk::epoch_info::EpochInfo> {
                unimplemented!()
            }
            async fn get_slot(&self) -> AggregatorResult<Slot> {
                unimplemented!()
            }
            async fn get_account(&self, _account_id: &str) -> AggregatorResult<Account> {
                unimplemented!()
            }
        }
        let publisher = Arc::new(Publisher::new(100));
        let dummy_blockchain = Arc::new(ErrorBlockchain);
        let dummy_storage = Arc::new(DummyStorage::new());
        let cancel_token = CancellationToken::new();
        let mut service = RpcIngestionService::new(
            dummy_blockchain,
            dummy_storage,
            publisher,
            None,
            3,
            cancel_token,
        );

        let result = service.run().await;
        assert!(result.is_ok());
        assert_eq!(service.last_processed_slot().load(Ordering::SeqCst), 0);
    }
}
