use crate::adapters::api::models::api_response::ApiResponse;
use crate::adapters::api::server::ApiState;
use crate::utils::error::AggregatorError;
use actix_web::{get, web, HttpResponse, Responder};
use chrono::{DateTime, NaiveDate, Utc};
use serde::Deserialize;
use std::sync::Arc;

/// Query parameters for transaction-related endpoints.
///
/// - `id`: Optional transaction identifier used for retrieving a single transaction.
/// - `day`: Optional date (in "dd/mm/yyyy" format) for which to retrieve transactions.
#[derive(Debug, Deserialize)]
pub struct TransactionQuery {
    pub id: Option<String>,
    pub day: Option<String>,
}

/// Retrieves and returns a formatted transaction based on the provided transaction ID.
///
/// This endpoint attempts to fetch the transaction from in-memory storage using the provided ID.
/// If the transaction is not found and a storage error occurs, it falls back to retrieving it from the blockchain,
/// then stores and returns it.
///
/// # Query Parameters
/// - `id`: The transaction identifier.
///
/// # Responses
/// - **200 OK**: Returns a JSON object containing the formatted transaction.
/// - **4XX/5XX**: Returns an error if the transaction ID is missing or if an error occurs during retrieval.
///
/// # Errors
/// - `AggregatorError::ApiError` if the transaction ID is not provided.
/// - Other `AggregatorError` variants may be returned if underlying storage or blockchain retrieval fails.
#[get("/transactions")]
pub async fn get_transaction(
    state: web::Data<Arc<ApiState>>,
    query: web::Query<TransactionQuery>,
) -> Result<impl Responder, AggregatorError> {
    if let Some(id) = &query.id {
        match state.storage.get_transaction(id).await {
            Ok(tx) => Ok(HttpResponse::Ok().json(ApiResponse::new(tx.to_formatted()))),
            Err(e) => {
                if let AggregatorError::StorageError(_) = e {
                    let tx = state.blockchain.get_transaction(id).await?;
                    state.storage.store_transaction(tx.clone()).await?;
                    Ok(HttpResponse::Ok().json(ApiResponse::new(tx.to_formatted())))
                } else {
                    Err(e)
                }
            }
        }
    } else {
        Err(AggregatorError::ApiError("Transaction ID required".into()))
    }
}

/// Retrieves and returns transactions for a specific date from in-memory storage.
///
/// NOTE
/// This endpoint does not trigger a network call to fetch transactions; it relies on the continuously
/// ingested transactions that are stored in memory. In a production setting, you might implement pagination
/// or streaming to improve performance and reduce memory usage.
///
/// # Query Parameters
/// - `day`: A string representing the date in the "dd/mm/yyyy" format.
///
/// # Responses
/// - **200 OK**: Returns a JSON object containing a list of formatted transactions, or a message indicating that
///   no transactions were found for the requested date.
/// - **4XX/5XX**: Returns an error if the date parameter is missing or in an invalid format.
///
/// # Errors
/// - `AggregatorError::ApiError` if the date is missing or if the date format is invalid.
/// - Other `AggregatorError` variants may be returned from the underlying storage call.
#[get("/transactions/by-date")]
pub async fn get_transactions_by_date(
    state: web::Data<Arc<ApiState>>,
    query: web::Query<TransactionQuery>,
) -> Result<impl Responder, AggregatorError> {
    if let Some(day) = &query.day {
        let date = NaiveDate::parse_from_str(day, "%d/%m/%Y")
            .map_err(|e| AggregatorError::ApiError(format!("Invalid date format: {}", e)))?;
        let naive = date.and_hms_opt(0, 0, 0).ok_or_else(|| {
            AggregatorError::ApiError("Invalid time, could not create NaiveDateTime".into())
        })?;
        let datetime = DateTime::from_naive_utc_and_offset(naive, Utc);

        let txs: Vec<_> = state
            .storage
            .get_transactions_by_date(datetime)
            .await?
            .iter()
            .map(|tx| tx.to_formatted())
            .collect();
        if !txs.is_empty() {
            Ok(HttpResponse::Ok().json(ApiResponse::new(txs)))
        } else {
            Ok(HttpResponse::Ok().json(ApiResponse::new(
                "No transactions found for the requested date. Transactions are currently being indexed, please try again later."
            )))
        }
    } else {
        Err(AggregatorError::ApiError("Date required".into()))
    }
}
