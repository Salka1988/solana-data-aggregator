use crate::core::domain::transaction::{Transaction, TransactionStatus};
use crate::utils::error::{AggregatorError, AggregatorResult};
use chrono::{DateTime, Utc};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::clock::Slot;
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::{pubkey::Pubkey, signature::Signature};
use solana_transaction_status::{
    EncodedTransactionWithStatusMeta, TransactionDetails, UiMessage, UiTransactionEncoding,
};
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;
use tokio_util::sync::CancellationToken;
use tracing::error;

/// Parses a Solana transaction using the provided block timestamp.
///
/// This function extracts the necessary metadata and account information from an
/// `EncodedTransactionWithStatusMeta` value. It performs the following steps:
/// - Checks that transaction metadata is present.
/// - Extracts the first signature from the JSON-encoded transaction.
/// - Retrieves account keys (either raw or parsed) and ensures there are at least two keys.
/// - Converts the first two account keys into sender and receiver `Pubkey` values.
/// - Calculates the transfer amount using the difference in pre- and post-balances and subtracting the fee.
/// - Constructs a `Transaction` object with a status of `Success` if the metadata status is OK,
///   or `Failed` otherwise.
///
/// # Arguments
///
/// * `tx` - The encoded transaction with its metadata.
/// * `timestamp` - A `DateTime<Utc>` value representing the block time.
///
/// # Returns
///
/// Returns an `AggregatorResult<Transaction>` on success or an error if any parsing step fails.
pub fn parse_transaction(
    tx: EncodedTransactionWithStatusMeta,
    timestamp: DateTime<Utc>,
) -> AggregatorResult<Transaction> {
    let meta = tx
        .meta
        .ok_or_else(|| AggregatorError::ProcessingError("Missing transaction metadata".into()))?;
    let signature = match &tx.transaction {
        solana_transaction_status::EncodedTransaction::Json(json_tx) => {
            if json_tx.signatures.is_empty() {
                return Err(AggregatorError::ProcessingError(
                    "No signatures in transaction".into(),
                ));
            }
            Signature::from_str(&json_tx.signatures[0]).map_err(|e| {
                AggregatorError::InvalidTransaction(format!("Failed to parse signature: {}", e))
            })?
        }
        _ => {
            return Err(AggregatorError::ProcessingError(
                "Unsupported transaction encoding".into(),
            ))
        }
    };

    let account_keys = match &tx.transaction {
        solana_transaction_status::EncodedTransaction::Json(json_tx) => match &json_tx.message {
            UiMessage::Raw(raw_msg) => raw_msg.account_keys.clone(),
            UiMessage::Parsed(parsed_msg) => parsed_msg
                .account_keys
                .iter()
                .map(|key| key.pubkey.clone())
                .collect(),
        },
        _ => {
            return Err(AggregatorError::ProcessingError(
                "Unsupported transaction encoding for account keys".into(),
            ))
        }
    };

    if account_keys.len() < 2 {
        return Err(AggregatorError::ProcessingError(
            "Insufficient account keys to determine sender and receiver".into(),
        ));
    }

    let sender = Pubkey::from_str(&account_keys[0])
        .map_err(|e| AggregatorError::ProcessingError(format!("Invalid sender pubkey: {}", e)))?;
    let receiver = Pubkey::from_str(&account_keys[1])
        .map_err(|e| AggregatorError::ProcessingError(format!("Invalid receiver pubkey: {}", e)))?;

    let sender_change = meta.pre_balances[0].saturating_sub(meta.post_balances[0]);
    let transfer_amount = sender_change.saturating_sub(meta.fee);

    Ok(Transaction::new(
        signature,
        timestamp,
        sender,
        receiver,
        transfer_amount,
        if meta.status.is_ok() {
            TransactionStatus::Success
        } else {
            TransactionStatus::Failed
        },
    ))
}

pub async fn parse_transaction_from_client(
    _client: &solana_client::nonblocking::rpc_client::RpcClient,
    tx: EncodedTransactionWithStatusMeta,
    timestamp: DateTime<Utc>,
    _slot: u64,
) -> AggregatorResult<Transaction> {
    parse_transaction(tx, timestamp)
}

/// A helper function that wraps `parse_transaction` for compatibility with a client interface.
///
/// This asynchronous function calls `parse_transaction` and returns its result. It exists to
/// provide a uniform interface, even if no client-specific logic is applied.
///
/// # Arguments
///
/// * `_client` - A reference to the RPC client (unused in this function).
/// * `tx` - The encoded transaction with status metadata.
/// * `timestamp` - The block timestamp as a `DateTime<Utc>`.
/// * `_slot` - The slot number (unused).
///
/// # Returns
///
/// Returns an `AggregatorResult<Transaction>` as produced by `parse_transaction`.
pub async fn process_block_transactions_for_slot(
    client: Arc<RpcClient>,
    slot: Slot,
) -> AggregatorResult<Vec<Transaction>> {
    let block = client
        .get_block_with_config(
            slot,
            solana_client::rpc_config::RpcBlockConfig {
                encoding: Some(UiTransactionEncoding::JsonParsed),
                transaction_details: Some(TransactionDetails::Full),
                rewards: Some(true),
                commitment: Some(CommitmentConfig::confirmed()),
                max_supported_transaction_version: Some(0),
            },
        )
        .await
        .map_err(|e| AggregatorError::BlockchainError(e.to_string()))?;

    let block_time = block
        .block_time
        .ok_or_else(|| AggregatorError::ProcessingError("No block time".into()))?;
    let naive = chrono::DateTime::from_timestamp(block_time, 0)
        .ok_or_else(|| AggregatorError::ProcessingError("Invalid block time".into()))?
        .naive_utc();
    let timestamp = chrono::DateTime::<Utc>::from_naive_utc_and_offset(naive, Utc);

    let mut transactions = Vec::new();
    if let Some(tx_list) = block.transactions {
        for tx in tx_list {
            match parse_transaction_from_client(&client, tx, timestamp, slot).await {
                Ok(parsed_tx) => transactions.push(parsed_tx),
                Err(e) => {
                    error!("Failed to parse transaction at slot {}: {}", slot, e);
                    return Err(e);
                }
            }
        }
    }
    Ok(transactions)
}

pub async fn retry_process_slot(
    client: Arc<RpcClient>,
    slot: Slot,
    cancel_token: CancellationToken,
    max_attempts: usize,
) -> AggregatorResult<Vec<Transaction>> {
    let mut attempts = 0;
    loop {
        if cancel_token.is_cancelled() {
            return Err(AggregatorError::ProcessingError("Cancelled".into()));
        }
        match process_block_transactions_for_slot(client.clone(), slot).await {
            Ok(txs) => return Ok(txs),
            Err(e) => {
                attempts += 1;
                if attempts >= max_attempts {
                    tracing::warn!(
                        "Slot {} failed after {} attempts: {}",
                        slot,
                        max_attempts,
                        e
                    );
                    return Err(e);
                } else {
                    tracing::warn!("Slot {} attempt {} failed; retrying...", slot, attempts);
                    tokio::time::sleep(Duration::from_secs(1)).await;
                }
            }
        }
    }
}
