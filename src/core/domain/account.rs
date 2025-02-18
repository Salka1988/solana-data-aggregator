use serde::{Deserialize, Serialize};
use solana_sdk::pubkey::Pubkey;

/// Represents an account on the Solana blockchain.
///
/// This struct holds the raw account data retrieved from the blockchain,
/// including the public key, balance in lamports, owner public key, whether the account
/// is executable, and the rent epoch.
///
/// # Fields
///
/// - `pubkey`: The public key identifying the account.
/// - `lamports`: The balance of the account in lamports (smallest unit of SOL).
/// - `owner`: The public key of the account's owner.
/// - `executable`: A flag indicating whether the account contains executable code.
/// - `rent_epoch`: The epoch in which the account last paid rent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Account {
    pub pubkey: Pubkey,
    pub lamports: u64,
    pub owner: Pubkey,
    pub executable: bool,
    pub rent_epoch: u64,
}

/// Represents a formatted view of an account suitable for API responses or human display.
///
/// This struct provides a more user-friendly representation of an account, converting numerical values
/// into formatted strings and renaming fields appropriately for presentation purposes.
#[derive(Serialize)]
pub struct FormattedAccount {
    /// The account's public key, represented as a string.
    #[serde(rename = "Public Key")]
    pub pubkey: String,
    /// The account balance formatted as a SOL amount (with 9 decimal places).
    #[serde(rename = "Balance")]
    pub balance: String,
    /// The owner of the account, represented as a string.
    #[serde(rename = "Owner")]
    pub owner: String,
    /// Indicates whether the account is executable.
    #[serde(rename = "Executable")]
    pub executable: bool,
    /// The rent epoch for the account.
    #[serde(rename = "Rent Epoch")]
    pub rent_epoch: u64,
}

impl Account {
    /// Converts a raw `Account` into a more user-friendly `FormattedAccount`.
    ///
    /// This method transforms the account's data into formatted strings. For example, the balance,
    /// originally represented in lamports, is converted to a SOL value with 9 decimal places.
    ///
    /// # Returns
    ///
    /// A `FormattedAccount` containing the account data in a presentation-friendly format.
    pub fn to_formatted(&self) -> FormattedAccount {
        FormattedAccount {
            pubkey: self.pubkey.to_string(),
            balance: format!("{:.9} SOL", self.lamports as f64 / 1_000_000_000.0),
            owner: self.owner.to_string(),
            executable: self.executable,
            rent_epoch: self.rent_epoch,
        }
    }
}
