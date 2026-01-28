//! Error types for near-contract-sim

use near_account_id::AccountId;
use near_gas::NearGas;
use thiserror::Error;

/// Errors that can occur during contract simulation
#[derive(Debug, Error)]
pub enum SimError {
    /// Contract not found at the specified account
    #[error("Contract not found at account '{account_id}'")]
    ContractNotFound { account_id: AccountId },

    /// Account not found
    #[error("Account not found: '{account_id}'")]
    AccountNotFound { account_id: AccountId },

    /// Method not found in contract
    #[error("Method '{method}' not found in contract '{account_id}'")]
    MethodNotFound { account_id: AccountId, method: String },

    /// Contract execution failed (panic or abort)
    #[error("Execution failed at {account_id}::{method}: {message}")]
    ExecutionError { account_id: AccountId, method: String, message: String },

    /// Ran out of gas during execution
    #[error("Out of gas: used {used}, limit {limit}")]
    OutOfGas { used: NearGas, limit: NearGas },

    /// Failed to deserialize arguments or return value
    #[error("Deserialization error: {message}")]
    DeserializationError { message: String },

    /// Invalid account ID format
    #[error("Invalid account ID: '{id}'")]
    InvalidAccountId { id: String },

    /// WASM compilation or preparation error
    #[error("WASM error: {message}")]
    WasmError { message: String },

    /// Storage error
    #[error("Storage error: {message}")]
    StorageError { message: String },

    /// IO error (file not found, etc.)
    #[error("IO error: {message}")]
    IoError { message: String },

    /// Insufficient balance for operation
    #[error("Insufficient balance: required {required}, available {available}")]
    InsufficientBalance { required: near_token::NearToken, available: near_token::NearToken },
}

impl SimError {
    pub fn contract_not_found(account_id: impl Into<AccountId>) -> Self {
        SimError::ContractNotFound { account_id: account_id.into() }
    }

    pub fn method_not_found(account_id: impl Into<AccountId>, method: impl Into<String>) -> Self {
        SimError::MethodNotFound { account_id: account_id.into(), method: method.into() }
    }

    pub fn execution_error(
        account_id: impl Into<AccountId>,
        method: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        SimError::ExecutionError {
            account_id: account_id.into(),
            method: method.into(),
            message: message.into(),
        }
    }
}

pub type SimResult<T> = Result<T, SimError>;
