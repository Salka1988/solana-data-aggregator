use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use solana_sdk::{pubkey::Pubkey, signature::Signature};

/// Represents a transaction on the Solana blockchain.
///
/// This struct contains all the necessary data related to a transaction,
/// including its signature, timestamp, sender, receiver, the amount transferred,
/// and the status of the transaction.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Transaction {
    /// The signature of the transaction.
    pub signature: Signature,
    /// The timestamp of when the transaction was recorded (in UTC).
    pub timestamp: DateTime<Utc>,
    /// The public key of the sender.
    pub sender: Pubkey,
    /// The public key of the receiver.
    pub receiver: Pubkey,
    /// The amount transferred in the transaction (in lamports).
    pub amount: u64,
    /// The status of the transaction (e.g. Success, Failed, Processing).
    pub status: TransactionStatus,
}

/// A formatted representation of a transaction for display or API responses.
///
/// This struct converts the raw transaction data into human-friendly formats.
/// For example, the timestamp is converted to an RFC 3339 string and the amount
/// is formatted as SOL with nine decimal places.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormattedTransaction {
    /// The transaction signature as a string.
    #[serde(rename = "Signature")]
    pub signature: String,
    /// The transaction timestamp formatted as an RFC 3339 string.
    #[serde(rename = "Timestamp")]
    pub timestamp: String,
    /// The sender's public key as a string.
    #[serde(rename = "Sender")]
    pub sender: String,
    /// The receiver's public key as a string.
    #[serde(rename = "Receiver")]
    pub receiver: String,
    /// The transaction amount formatted as SOL.
    #[serde(rename = "Amount")]
    pub amount: String,
    /// The status of the transaction as a string.
    #[serde(rename = "Status")]
    pub status: String,
}

impl Transaction {
    /// Converts a `Transaction` into its formatted representation.
    ///
    /// This method transforms raw transaction data into a more user-friendly
    /// format suitable for API responses or display. For example, it converts the
    /// timestamp to RFC 3339 format and converts the amount from lamports to SOL.
    ///
    /// # Returns
    ///
    /// A `FormattedTransaction` containing the formatted values.
    pub fn to_formatted(&self) -> FormattedTransaction {
        FormattedTransaction {
            signature: self.signature.to_string(),
            timestamp: self.timestamp.to_rfc3339(),
            sender: self.sender.to_string(),
            receiver: self.receiver.to_string(),
            amount: self.amount.to_string(),
            status: format!("{:?}", self.status),
        }
    }
}

/// Represents the status of a transaction.
///
/// This enum indicates whether a transaction was successful, failed,
/// or is still processing. The default status is `Processing`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub enum TransactionStatus {
    /// The transaction completed successfully.
    Success,
    /// The transaction failed.
    Failed,
    /// The transaction is still being processed.
    #[default]
    Processing,
}

impl Transaction {
    /// Creates a new `Transaction` instance.
    ///
    /// # Arguments
    ///
    /// * `signature` - The signature of the transaction.
    /// * `timestamp` - The timestamp when the transaction occurred (in UTC).
    /// * `sender` - The public key of the sender.
    /// * `receiver` - The public key of the receiver.
    /// * `amount` - The amount transferred in the transaction (in lamports).
    /// * `status` - The status of the transaction.
    ///
    /// # Returns
    ///
    /// A new instance of `Transaction` populated with the provided data.
    pub fn new(
        signature: Signature,
        timestamp: DateTime<Utc>,
        sender: Pubkey,
        receiver: Pubkey,
        amount: u64,
        status: TransactionStatus,
    ) -> Self {
        Self {
            signature,
            timestamp,
            sender,
            receiver,
            amount,
            status,
        }
    }
}
