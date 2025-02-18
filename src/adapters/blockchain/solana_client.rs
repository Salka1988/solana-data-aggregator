use crate::adapters::blockchain::{solana_stream, transaction_parser};
use crate::core::domain::account::Account;
use crate::core::domain::transaction::Transaction;
use crate::core::ports::blockchain::BlockchainPort;
use crate::metrics::connection_health_tracker::ConnectionHealthTracker;
use crate::utils::error::{AggregatorError, AggregatorResult};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{
    clock::Slot, commitment_config::CommitmentConfig, pubkey::Pubkey, signature::Signature,
};
use solana_transaction_status::{TransactionDetails, UiTransactionEncoding};
use std::pin::Pin;
use std::str::FromStr;
use std::sync::atomic::AtomicU64;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;
use tracing::error;

/// Adapter for interacting with the Solana blockchain via an RPC endpoint.
///
/// This adapter wraps a non-blocking RPC client and provides additional state
/// and functionality required for blockchain data ingestion. It keeps track of
/// the last processed slot and monitors the connection health using a dedicated tracker.
///
/// # Fields
///
/// * `client` - An `Arc`-wrapped instance of `RpcClient` used for communicating with the Solana blockchain.
/// * `last_processed_slot` - An atomic counter that stores the most recent slot number that has been processed.
/// * `health_tracker` - A `ConnectionHealthTracker` that monitors the health and reliability of the connection to the blockchain.
pub struct SolanaClientAdapter {
    client: Arc<RpcClient>,
    last_processed_slot: AtomicU64,
    health_tracker: ConnectionHealthTracker,
}

impl SolanaClientAdapter {
    pub fn new(url: &str, unhealthy_after_n_errors: usize) -> Self {
        let client = Arc::new(RpcClient::new_with_commitment(
            url.to_string(),
            CommitmentConfig::confirmed(),
        ));
        Self {
            client,
            last_processed_slot: AtomicU64::new(0),
            health_tracker: ConnectionHealthTracker::new(unhealthy_after_n_errors),
        }
    }

    pub(super) fn parse_timestamp(&self, block_time: i64) -> AggregatorResult<DateTime<Utc>> {
        let naive = chrono::DateTime::from_timestamp(block_time, 0)
            .ok_or_else(|| AggregatorError::ProcessingError("Invalid block time".into()))?
            .naive_utc();
        Ok(DateTime::from_naive_utc_and_offset(naive, Utc))
    }

    /// Retrieves and parses transactions for a given slot.
    pub async fn process_block_transactions(
        &self,
        slot: Slot,
    ) -> AggregatorResult<Vec<Transaction>> {
        let block = self
            .client
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
        let timestamp = self.parse_timestamp(block_time)?;

        let mut transactions = Vec::new();
        if let Some(tx_list) = block.transactions {
            for tx in tx_list {
                match transaction_parser::parse_transaction(tx, timestamp) {
                    Ok(parsed_tx) => transactions.push(parsed_tx),
                    Err(e) => error!("Failed to parse transaction at slot {}: {}", slot, e),
                }
            }
        }
        Ok(transactions)
    }

    pub fn client(&self) -> &Arc<RpcClient> {
        &self.client
    }

    pub fn last_processed_slot(&self) -> &AtomicU64 {
        &self.last_processed_slot
    }

    pub fn health_tracker(&self) -> &ConnectionHealthTracker {
        &self.health_tracker
    }
}

#[async_trait]
impl BlockchainPort for SolanaClientAdapter {
    /// Retrieves a transaction from the Solana blockchain using its signature.
    ///
    /// The method first converts the provided signature string into a `Signature` type.
    /// Then, it calls the RPC client with a specific transaction configuration.
    /// If the RPC call succeeds but the transaction's metadata is missing, an error is returned.
    /// Otherwise, the block time is converted into a `DateTime<Utc>` using the helper method,
    /// and the raw transaction is parsed into a `Transaction` object.
    ///
    /// # Arguments
    ///
    /// * `signature` - A string representing the transaction signature.
    ///
    /// # Returns
    ///
    /// An `AggregatorResult<Transaction>` containing the parsed transaction on success,
    /// or an error variant (`InvalidTransaction` or `BlockchainError`) on failure.
    async fn get_transaction(&self, signature: &str) -> AggregatorResult<Transaction> {
        let signature = Signature::from_str(signature).map_err(|e| {
            AggregatorError::InvalidTransaction(format!("Invalid signature format: {}", e))
        })?;

        let tx_result = self
            .client
            .get_transaction_with_config(
                &signature,
                solana_client::rpc_config::RpcTransactionConfig {
                    encoding: Some(UiTransactionEncoding::JsonParsed),
                    commitment: Some(CommitmentConfig::confirmed()),
                    max_supported_transaction_version: Some(0),
                },
            )
            .await;

        match tx_result {
            Ok(tx) => {
                if tx.transaction.meta.is_none() {
                    return Err(AggregatorError::InvalidTransaction(
                        "Transaction not found".into(),
                    ));
                }

                let block_time = tx.block_time.ok_or_else(|| {
                    AggregatorError::ProcessingError("No block time available".into())
                })?;
                let timestamp = self.parse_timestamp(block_time)?;

                transaction_parser::parse_transaction(tx.transaction, timestamp).map_err(|e| {
                    AggregatorError::InvalidTransaction(format!(
                        "Failed to parse transaction: {}",
                        e
                    ))
                })
            }
            Err(e) => {
                let error_str = e.to_string();
                if error_str.contains("invalid type: null")
                    || error_str.contains("Transaction not found")
                {
                    Err(AggregatorError::InvalidTransaction(
                        "Transaction not found".into(),
                    ))
                } else {
                    Err(AggregatorError::BlockchainError(format!(
                        "Failed to fetch transaction: {}",
                        e
                    )))
                }
            }
        }
    }

    /// Streams transactions for the current epoch starting from a given slot.
    ///
    /// This method delegates the streaming logic to `solana_stream::stream_current_epoch_transactions`,
    /// passing along the RPC client, cancellation token, last processed slot, and an optional batch size.
    /// It returns a pinned stream of results, where each item is a tuple of a slot number and a vector of parsed transactions.
    ///
    /// # Arguments
    ///
    /// * `cancel_token` - A `CancellationToken` used to cancel the stream.
    /// * `last_processed_slot` - The last processed slot, from which to continue streaming.
    /// * `batch_size` - An optional number specifying the maximum number of transactions per batch.
    ///
    /// # Returns
    ///
    /// An `AggregatorResult` containing a pinned stream that yields either a successful (Slot, Vec<Transaction>) tuple or an error.
    async fn stream_current_epoch_transactions(
        &self,
        cancel_token: CancellationToken,
        last_processed_slot: u64,
        batch_size: Option<usize>,
        max_retry_attempts: usize,
    ) -> AggregatorResult<
        Pin<
            Box<dyn futures_util::Stream<Item = AggregatorResult<(Slot, Vec<Transaction>)>> + Send>,
        >,
    > {
        solana_stream::stream_current_epoch_transactions(
            self.client.clone(),
            cancel_token,
            last_processed_slot,
            batch_size,
            max_retry_attempts,
        )
        .await
    }

    /// Retrieves the current epoch information from the Solana RPC endpoint.
    ///
    /// # Returns
    ///
    /// An `AggregatorResult` containing the epoch information on success,
    /// or a `BlockchainError` if the RPC call fails.
    async fn get_epoch_info(&self) -> AggregatorResult<solana_sdk::epoch_info::EpochInfo> {
        self.client
            .get_epoch_info()
            .await
            .map_err(|e| AggregatorError::BlockchainError(e.to_string()))
    }

    /// Retrieves the current slot number from the Solana RPC endpoint.
    ///
    /// # Returns
    ///
    /// An `AggregatorResult` containing the current slot on success,
    /// or a `BlockchainError` if the RPC call fails.
    async fn get_slot(&self) -> AggregatorResult<Slot> {
        self.client
            .get_slot()
            .await
            .map_err(|e| AggregatorError::BlockchainError(e.to_string()))
    }

    /// Retrieves account information for the given account ID.
    ///
    /// The method first validates the provided account ID by converting it into a `Pubkey`.
    /// If successful, it queries the account data from the RPC client with confirmed commitment.
    /// On success, an `Account` object is returned; on failure, an appropriate error is generated.
    ///
    /// # Arguments
    ///
    /// * `account_id` - A string representing the account's public key.
    ///
    /// # Returns
    ///
    /// An `AggregatorResult<Account>` with the account information on success,
    /// or an error (`ProcessingError` or `BlockchainError`) if the account data cannot be retrieved.
    async fn get_account(&self, account_id: &str) -> AggregatorResult<Account> {
        let pubkey = Pubkey::from_str(account_id)
            .map_err(|e| AggregatorError::ProcessingError(format!("Invalid account id: {}", e)))?;

        match self
            .client
            .get_account_with_commitment(&pubkey, CommitmentConfig::confirmed())
            .await
        {
            Ok(response) => {
                let account_info = response
                    .value
                    .ok_or_else(|| AggregatorError::ProcessingError("Account not found".into()))?;

                Ok(Account {
                    pubkey,
                    lamports: account_info.lamports,
                    owner: account_info.owner,
                    executable: account_info.executable,
                    rent_epoch: account_info.rent_epoch,
                })
            }
            Err(e) => {
                let error_str = e.to_string();
                if error_str.contains("invalid type: null")
                    || error_str.contains("Account not found")
                {
                    Err(AggregatorError::ProcessingError("Account not found".into()))
                } else {
                    Err(AggregatorError::BlockchainError(format!(
                        "Failed to fetch account: {}",
                        e
                    )))
                }
            }
        }
    }
}
