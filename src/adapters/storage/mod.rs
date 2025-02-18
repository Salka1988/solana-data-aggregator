pub mod memory_storage;

#[cfg(test)]
mod tests {
    use crate::adapters::storage::memory_storage::MemoryStorageAdapter;
    use crate::core::domain::transaction::{Transaction, TransactionStatus};
    use crate::core::ports::storage::StoragePort;
    use crate::utils::error::AggregatorError;
    use chrono::{TimeZone, Utc};
    use solana_sdk::{pubkey::Pubkey, signature::Signature};

    fn create_dummy_transaction(
        signature_bytes: [u8; 64],
        timestamp: chrono::DateTime<Utc>,
        amount: u64,
    ) -> Transaction {
        let signature = Signature::from(signature_bytes);
        let sender = Pubkey::new_unique();
        let receiver = Pubkey::new_unique();
        Transaction::new(
            signature,
            timestamp,
            sender,
            receiver,
            amount,
            TransactionStatus::Success,
        )
    }

    #[tokio::test]
    async fn test_store_and_get_transaction() {
        let adapter = MemoryStorageAdapter::new();
        let timestamp = Utc::now();
        let tx = create_dummy_transaction([1; 64], timestamp, 100);

        adapter
            .store_transaction(tx.clone())
            .await
            .expect("Failed to store transaction");
        let stored_tx = adapter
            .get_transaction(&tx.signature.to_string())
            .await
            .expect("Transaction not found");

        assert_eq!(stored_tx.signature.to_string(), tx.signature.to_string());
        assert_eq!(stored_tx.timestamp, tx.timestamp);
    }

    #[tokio::test]
    async fn test_get_transaction_not_found() {
        let adapter = MemoryStorageAdapter::new();
        let result = adapter.get_transaction("nonexistent_signature").await;
        assert!(
            result.is_err(),
            "Expected error for non-existent transaction"
        );

        if let Err(AggregatorError::StorageError(msg)) = result {
            assert!(msg.contains("Transaction not found"));
        } else {
            panic!("Expected a StorageError");
        }
    }

    #[tokio::test]
    async fn test_get_transactions_by_date() {
        let adapter = MemoryStorageAdapter::new();

        let timestamp1 = Utc.with_ymd_and_hms(2025, 2, 17, 12, 0, 0).unwrap();
        let timestamp2 = Utc.with_ymd_and_hms(2025, 2, 17, 15, 0, 0).unwrap();
        let timestamp3 = Utc.with_ymd_and_hms(2025, 2, 18, 10, 0, 0).unwrap();

        let tx1 = create_dummy_transaction([2; 64], timestamp1, 100);
        let tx2 = create_dummy_transaction([3; 64], timestamp2, 200);
        let tx3 = create_dummy_transaction([4; 64], timestamp3, 300);

        adapter
            .store_transaction(tx1.clone())
            .await
            .expect("Failed to store tx1");
        adapter
            .store_transaction(tx2.clone())
            .await
            .expect("Failed to store tx2");
        adapter
            .store_transaction(tx3.clone())
            .await
            .expect("Failed to store tx3");

        let txs_day1 = adapter
            .get_transactions_by_date(timestamp1)
            .await
            .expect("Failed to get transactions by date");
        assert_eq!(
            txs_day1.len(),
            2,
            "Expected 2 transactions for Feb 17, 2025"
        );

        // Retrieve transactions for February 18, 2025.
        let txs_day2 = adapter
            .get_transactions_by_date(timestamp3)
            .await
            .expect("Failed to get transactions by date");
        assert_eq!(txs_day2.len(), 1, "Expected 1 transaction for Feb 18, 2025");
    }
}
