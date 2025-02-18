use crate::adapters::api::models::api_response::ApiResponse;
use crate::adapters::api::server::ApiState;
use crate::utils::error::AggregatorError;
use actix_web::{get, web, HttpResponse, Responder};
use serde::Deserialize;
use std::sync::Arc;

/// Query parameters for the account endpoint.
///
/// # Fields
///
/// - `id`: A string representing the account identifier (typically a public key).
#[derive(Debug, Deserialize)]
pub struct AccountQuery {
    pub id: String,
}

/// Retrieves and returns account information for a given account ID.
///
/// This endpoint calls the blockchain port from the API state to fetch the account details,
/// and then returns the information in a formatted JSON response.
///
/// # Query Parameters
///
/// - `id`: The account identifier to look up.
///
/// # Responses
///
/// - **200 OK**: Returns a JSON object containing the formatted account information.
/// - **4XX/5XX**: Returns an error if the account could not be retrieved.
///
/// # Errors
///
/// Returns an `AggregatorError` if the account lookup fails.
#[get("/accounts")]
pub async fn get_account(
    state: web::Data<Arc<ApiState>>,
    query: web::Query<AccountQuery>,
) -> Result<impl Responder, AggregatorError> {
    let account = state.blockchain.get_account(&query.id).await?;
    Ok(HttpResponse::Ok().json(ApiResponse::new(account.to_formatted())))
}
