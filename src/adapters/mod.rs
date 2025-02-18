use crate::core::domain::transaction::Transaction;
use crate::utils::error::AggregatorResult;
use actix::dev::Stream;

pub mod api;
pub mod blockchain;
pub mod health_tracking_adapter;
pub mod storage;
type TxStream = Box<dyn Stream<Item = AggregatorResult<Vec<Transaction>>> + Send>;
