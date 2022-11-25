use cosmwasm_std::{DecimalRangeExceeded, OverflowError, StdError};
use serde_json_wasm;
use thiserror::Error;

pub type NeutronResult<T> = Result<T, NeutronError>;

#[derive(Error, Debug, PartialEq)]
pub enum NeutronError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("{0}")]
    Fmt(#[from] std::fmt::Error),

    #[error("Serde JSON (Wasm) error")]
    SerdeJSONWasm(String),

    #[error("address length should be max {max:?} bytes, got {actual:?}")]
    MaxAddrLength { max: usize, actual: usize },

    #[error("Decimal range exceeded")]
    DecimalRangeExceeded(#[from] DecimalRangeExceeded),

    #[error("Overflow error")]
    OverflowError(#[from] OverflowError),
}

impl From<serde_json_wasm::de::Error> for NeutronError {
    fn from(e: serde_json_wasm::de::Error) -> Self {
        NeutronError::SerdeJSONWasm(e.to_string())
    }
}
