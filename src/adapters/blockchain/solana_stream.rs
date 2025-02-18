use crate::adapters::blockchain::transaction_parser::retry_process_slot;
use crate::core::domain::transaction::Transaction;
use crate::utils::error::{AggregatorError, AggregatorResult};
use futures_util::{Stream, StreamExt};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::clock::Slot;
use solana_sdk::epoch_info::EpochInfo;
use std::pin::Pin;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

type SlotTxStream = Box<dyn Stream<Item = AggregatorResult<(Slot, Vec<Transaction>)>> + Send>;

/// Streams transactions for the current epoch from a Solana RPC client.
///
/// This asynchronous function creates a stream of transaction batches across slots, starting
/// from the appropriate starting slot (either the first slot of the current epoch or the slot
/// immediately following the last processed slot) up to the current slot. For each slot in this
/// range, it processes the block’s transactions using `process_block_transactions_for_slot`.
///
/// If a batch size is provided, the transactions for each slot are grouped into batches of the given
/// size; otherwise, all transactions from a slot are returned together. Empty transaction results are
/// filtered out. The stream yields items of type `AggregatorResult<(Slot, Vec<Transaction>)>`, where each
/// item is a tuple containing the slot number and a vector of transactions parsed from that slot.
///
/// # Parameters
///
/// - `client`: An `Arc` reference to an `RpcClient` for communicating with the Solana blockchain.
/// - `cancel_token`: A `CancellationToken` that allows for cancellation of the streaming operation.
/// - `last_processed_slot`: The slot number that was last successfully processed.
///   If this value is less than the first slot of the current epoch, processing will start from the
///   first slot of the current epoch; otherwise, processing starts from `last_processed_slot + 1`.
/// - `batch_size`: An optional parameter specifying the maximum number of transactions to include in
///   each batch. When provided, transactions for each slot are chunked into batches of this size.
/// - `max_retry_attempts`: The maximum number of retry attempts for processing a slot.
///   If not provided, the default value of 5 is used.
/// # Returns
///
/// Returns an `AggregatorResult` wrapping a pinned stream that yields items of type
/// `AggregatorResult<(Slot, Vec<Transaction>)>`. Each stream item represents a slot and its corresponding
/// batch (or batches) of transactions. Errors occurring during epoch info retrieval, slot fetching,
/// cancellation, or transaction parsing are propagated as appropriate.
///
/// # Errors
///
/// - If retrieving epoch information or the current slot fails, a `BlockchainError` is returned.
/// - If the cancellation token is triggered during processing, a `ProcessingError("Cancelled")` error is returned
///   for the affected slot.
/// - Errors from parsing transactions or other processing steps are wrapped and returned.
///
/// # Example
///
/// ```no_run
/// use std::sync::Arc;
/// use tokio_util::sync::CancellationToken;
/// use solana_client::nonblocking::rpc_client::RpcClient;
/// use solana_sdk::clock::Slot;///
/// #
/// use solana_data_aggregator::adapters::blockchain::solana_stream::stream_current_epoch_transactions;
///
/// async fn example() {
/// let client = Arc::new(RpcClient::new("https://api.devnet.solana.com".to_string()));
/// let cancel_token = CancellationToken::new();
/// let last_processed_slot: Slot = 100;
/// let batch_size = Some(10);
/// let max_retry = 3;
///
///
/// let stream = stream_current_epoch_transactions(client, cancel_token, last_processed_slot, batch_size, max_retry)
///     .await
///     .expect("Failed to stream transactions");
///
/// // Process the stream (e.g., with stream.next().await)
/// # }
/// ```
pub async fn stream_current_epoch_transactions(
    client: Arc<RpcClient>,
    cancel_token: CancellationToken,
    last_processed_slot: u64,
    batch_size: Option<usize>,
    max_retry_attempts: usize,
) -> AggregatorResult<Pin<SlotTxStream>> {
    let epoch_info: EpochInfo = client
        .get_epoch_info()
        .await
        .map_err(|e| AggregatorError::BlockchainError(e.to_string()))?;
    let first_slot_current_epoch = epoch_info.absolute_slot - epoch_info.slot_index;

    let start_slot = if last_processed_slot < first_slot_current_epoch {
        first_slot_current_epoch
    } else {
        last_processed_slot + 1
    };

    let current_slot = client
        .get_slot()
        .await
        .map_err(|e| AggregatorError::BlockchainError(e.to_string()))?;

    let slot_stream = futures_util::stream::iter(start_slot..=current_slot)
        .then({
            let cancel = cancel_token.clone();
            let client = client.clone();
            move |slot| {
                let cancel = cancel.clone();
                let client = client.clone();
                async move {
                    if cancel.is_cancelled() {
                        Err(AggregatorError::ProcessingError("Cancelled".into()))
                    } else {
                        retry_process_slot(client, slot, cancel, max_retry_attempts)
                            .await
                            .map(|txs| (slot, txs))
                    }
                }
            }
        })
        .filter_map(|res: AggregatorResult<(Slot, Vec<Transaction>)>| async {
            match res {
                Ok((slot, txs)) if !txs.is_empty() => Some(Ok((slot, txs))),
                Ok(_) => None,
                Err(e) => Some(Err(e)),
            }
        });

    let batched_stream: Pin<SlotTxStream> = if let Some(size) = batch_size {
        Box::pin(slot_stream.flat_map(move |res| match res {
            Ok((slot, txs)) => {
                let slot_for_chunks = slot;
                let chunked_stream = futures_util::stream::iter(txs.into_iter().map(Ok))
                    .chunks(size)
                    .map(move |chunk| {
                        match chunk
                            .into_iter()
                            .collect::<AggregatorResult<Vec<Transaction>>>()
                        {
                            Ok(batch) => Ok((slot_for_chunks, batch)),
                            Err(e) => Err(e),
                        }
                    });
                Box::pin(chunked_stream) as Pin<SlotTxStream>
            }
            Err(e) => Box::pin(futures_util::stream::iter(vec![Err(e)])) as Pin<SlotTxStream>,
        }))
    } else {
        Box::pin(slot_stream)
    };

    Ok(batched_stream)
}
