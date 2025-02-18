// Note: This test suite uses mock implementations (DummyBlockchain and DummyStorage)
// for simplicity and ease of testing. Future improvements could include integration
// with solana-test-validator for more comprehensive testing, but this would require:
// - Docker setup
// - solana-test-validator configuration
// - Additional test utilities
// These improvements can be added as needed, but aren't necessary for current test coverage.

use actix_web::{test, web, App};
use chrono::{Datelike, Utc};
use solana_data_aggregator::{
    adapters::api::handlers::transactions::{get_transaction, get_transactions_by_date},
    adapters::api::server::ApiState,
    core::{
        domain::{
            account::Account,
            transaction::{Transaction, TransactionStatus},
        },
        ports::{blockchain::BlockchainPort, storage::StoragePort},
    },
    utils::error::AggregatorError,
};
use solana_sdk::pubkey::ParsePubkeyError;
use std::sync::Arc;

struct DummyBlockchain;

#[async_trait::async_trait]
impl BlockchainPort for DummyBlockchain {
    async fn get_transaction(&self, _signature: &str) -> Result<Transaction, AggregatorError> {
        let now = Utc::now();
        let sender = "11111111111111111111111111111111".parse().unwrap();
        let receiver = "22222222222222222222222222222222".parse().unwrap();
        let signature = "1111111111111111111111111111111111111111111111111111111111111111"
            .parse()
            .unwrap();
        Ok(Transaction::new(
            signature,
            now,
            sender,
            receiver,
            1000,
            TransactionStatus::Success,
        ))
    }

    async fn stream_current_epoch_transactions(
        &self,
        _cancel_token: tokio_util::sync::CancellationToken,
        _last_processed_slot: u64,
        _batch_size: Option<usize>,
        _max_retry_attempts: usize,
    ) -> Result<
        std::pin::Pin<
            Box<
                dyn futures_util::Stream<Item = Result<(u64, Vec<Transaction>), AggregatorError>>
                    + Send,
            >,
        >,
        AggregatorError,
    > {
        use futures_util::stream;
        let now = Utc::now();
        let sender = "11111111111111111111111111111111".parse().unwrap();
        let receiver = "22222222222222222222222222222222".parse().unwrap();
        let signature = "1111111111111111111111111111111111111111111111111111111111111111"
            .parse()
            .unwrap();
        let tx = Transaction::new(
            signature,
            now,
            sender,
            receiver,
            1000,
            TransactionStatus::Success,
        );
        let s = stream::iter(vec![Ok((1, vec![tx]))]);
        Ok(Box::pin(s))
    }

    async fn get_epoch_info(&self) -> Result<solana_sdk::epoch_info::EpochInfo, AggregatorError> {
        Err(AggregatorError::ApiError("Not implemented".into()))
    }

    async fn get_slot(&self) -> Result<u64, AggregatorError> {
        Err(AggregatorError::ApiError("Not implemented".into()))
    }

    async fn get_account(&self, account_id: &str) -> Result<Account, AggregatorError> {
        let pubkey = account_id
            .parse()
            .map_err(|e: ParsePubkeyError| AggregatorError::ProcessingError(e.to_string()))?;
        Ok(Account {
            pubkey,
            lamports: 5_000_000_000,
            owner: "33333333333333333333333333333333".parse().unwrap(),
            executable: false,
            rent_epoch: 42,
        })
    }
}

struct DummyStorage;

#[async_trait::async_trait]
impl StoragePort for DummyStorage {
    async fn store_transaction(&self, _transaction: Transaction) -> Result<(), AggregatorError> {
        Ok(())
    }
    async fn get_transaction(&self, _signature: &str) -> Result<Transaction, AggregatorError> {
        let now = Utc::now();
        let sender = "11111111111111111111111111111111".parse().unwrap();
        let receiver = "11111111111111111111111111111112".parse().unwrap();
        let signature = "1111111111111111111111111111111111111111111111111111111111111111"
            .parse()
            .unwrap();
        Ok(Transaction::new(
            signature,
            now,
            sender,
            receiver,
            1000,
            TransactionStatus::Success,
        ))
    }
    async fn get_transactions_by_date(
        &self,
        _date: chrono::DateTime<Utc>,
    ) -> Result<Vec<Transaction>, AggregatorError> {
        let now = Utc::now();
        let sender = "11111111111111111111111111111111".parse().unwrap();
        let receiver = "11111111111111111111111111111112".parse().unwrap();
        let signature = "1111111111111111111111111111111111111111111111111111111111111111"
            .parse()
            .unwrap();
        Ok(vec![Transaction::new(
            signature,
            now,
            sender,
            receiver,
            1000,
            TransactionStatus::Success,
        )])
    }
    async fn stream_all_transactions_chunks(
        &self,
        _chunk_size: usize,
    ) -> std::pin::Pin<
        Box<dyn futures_util::Stream<Item = Result<Vec<Transaction>, AggregatorError>> + Send>,
    > {
        use futures_util::stream;
        let now = Utc::now();
        let sender = "11111111111111111111111111111111".parse().unwrap();
        let receiver = "11111111111111111111111111111112".parse().unwrap();
        let signature = "1111111111111111111111111111111111111111111111111111111111111111"
            .parse()
            .unwrap();
        let tx = Transaction::new(
            signature,
            now,
            sender,
            receiver,
            1000,
            TransactionStatus::Success,
        );
        Box::pin(stream::iter(vec![Ok(vec![tx])]))
    }
}

fn init_app_state() -> Arc<ApiState> {
    Arc::new(ApiState {
        blockchain: Arc::new(DummyBlockchain),
        storage: Arc::new(DummyStorage),
    })
}

#[actix_web::test]
async fn test_get_transaction_endpoint() {
    let state = init_app_state();
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(state))
            .service(get_transaction),
    )
    .await;

    let req = test::TestRequest::get()
        .uri("/transactions?id=1111111111111111111111111111111111111111111111111111111111111111")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), actix_web::http::StatusCode::OK);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["status"].as_str().unwrap(), "success");
}

#[actix_web::test]
async fn test_get_transactions_by_date_endpoint() {
    let state = init_app_state();
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(state))
            .service(get_transactions_by_date),
    )
    .await;

    // Use today's date in dd/mm/yyyy format.
    let today = Utc::now().date_naive();
    let date_str = format!(
        "{:02}/{:02}/{:04}",
        today.day(),
        today.month(),
        today.year()
    );
    let req = test::TestRequest::get()
        .uri(&format!("/transactions/by-date?day={}", date_str))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), actix_web::http::StatusCode::OK);
    let body: serde_json::Value = test::read_body_json(resp).await;
    if body["data"].is_array() {
        assert!(!body["data"].as_array().unwrap().is_empty());
    } else {
        assert!(body["data"]
            .as_str()
            .unwrap()
            .contains("No transactions found"));
    }
}
