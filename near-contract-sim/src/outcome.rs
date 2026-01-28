//! Types representing execution outcomes

use near_account_id::AccountId;
use near_gas::NearGas;
use near_token::NearToken;
use serde::de::DeserializeOwned;

use crate::error::{SimError, SimResult};

/// Status of a single receipt execution
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExecutionStatus {
    /// Execution completed successfully
    Success,
    /// Execution failed with an error message
    Failure(String),
    /// Contract panicked
    Panic(String),
}

impl ExecutionStatus {
    pub fn is_success(&self) -> bool {
        matches!(self, ExecutionStatus::Success)
    }

    pub fn is_failure(&self) -> bool {
        !self.is_success()
    }
}

/// Outcome of executing a single receipt
#[derive(Debug, Clone)]
pub struct ReceiptOutcome {
    /// Account that initiated this call
    pub predecessor: AccountId,
    /// Account that received this call
    pub receiver: AccountId,
    /// Method that was called
    pub method: String,
    /// Arguments passed to the method
    pub args: Vec<u8>,
    /// NEAR tokens attached to this call
    pub deposit: NearToken,
    /// Gas used by this execution
    pub gas_used: NearGas,
    /// Execution status
    pub status: ExecutionStatus,
    /// Logs emitted during execution
    pub logs: Vec<String>,
    /// Return value (if successful)
    pub return_value: Option<Vec<u8>>,
}

impl ReceiptOutcome {
    pub fn is_success(&self) -> bool {
        self.status.is_success()
    }

    pub fn is_failure(&self) -> bool {
        self.status.is_failure()
    }
}

/// Gas usage breakdown
#[derive(Debug, Clone, Default)]
pub struct GasProfile {
    /// Total gas used
    pub total: NearGas,
    /// Gas used for compute operations
    pub compute: NearGas,
    /// Gas used for storage reads
    pub storage_read: NearGas,
    /// Gas used for storage writes
    pub storage_write: NearGas,
}

/// Result of a complete call execution (including all cross-contract calls)
#[derive(Debug, Clone)]
pub struct CallOutcome {
    /// All receipt executions in order
    receipts: Vec<ReceiptOutcome>,
    /// Final return value
    return_value: Option<Vec<u8>>,
    /// Total gas used across all receipts
    total_gas_used: NearGas,
}

impl CallOutcome {
    pub fn new(receipts: Vec<ReceiptOutcome>) -> Self {
        let total_gas_used = receipts.iter().fold(NearGas::from_gas(0), |acc, r| {
            NearGas::from_gas(acc.as_gas() + r.gas_used.as_gas())
        });

        // Return value is from the first receipt (the initial call)
        let return_value = receipts.first().and_then(|r| r.return_value.clone());

        Self { receipts, return_value, total_gas_used }
    }

    /// Check if the overall execution was successful
    pub fn is_success(&self) -> bool {
        self.receipts.iter().all(|r| r.is_success())
    }

    /// Check if any execution failed
    pub fn is_failure(&self) -> bool {
        !self.is_success()
    }

    /// Get the return value as bytes
    pub fn return_value(&self) -> Option<&[u8]> {
        self.return_value.as_deref()
    }

    /// Deserialize the return value as JSON
    pub fn json<T: DeserializeOwned>(&self) -> SimResult<T> {
        let bytes = self.return_value.as_ref().ok_or_else(|| SimError::DeserializationError {
            message: "No return value".to_string(),
        })?;

        serde_json::from_slice(bytes)
            .map_err(|e| SimError::DeserializationError { message: e.to_string() })
    }

    /// Get all logs from all receipts
    pub fn logs(&self) -> Vec<&str> {
        self.receipts.iter().flat_map(|r| r.logs.iter().map(|s| s.as_str())).collect()
    }

    /// Get total gas used across all receipts
    pub fn gas_used(&self) -> NearGas {
        self.total_gas_used
    }

    /// Get the execution trace (all receipts)
    pub fn receipts(&self) -> &[ReceiptOutcome] {
        &self.receipts
    }

    /// Get the first receipt (initial call)
    pub fn primary_receipt(&self) -> Option<&ReceiptOutcome> {
        self.receipts.first()
    }

    /// Get the error message if execution failed
    pub fn error(&self) -> Option<&str> {
        self.receipts.iter().find_map(|r| match &r.status {
            ExecutionStatus::Failure(msg) => Some(msg.as_str()),
            ExecutionStatus::Panic(msg) => Some(msg.as_str()),
            ExecutionStatus::Success => None,
        })
    }

    // Assertion helpers

    /// Assert that execution was successful
    pub fn assert_success(&self) {
        if !self.is_success() {
            panic!(
                "Expected success but got failure: {:?}",
                self.error().unwrap_or("unknown error")
            );
        }
    }

    /// Assert that execution failed
    pub fn assert_failure(&self) {
        if !self.is_failure() {
            panic!("Expected failure but execution succeeded");
        }
    }

    /// Assert that execution failed with a message containing the given string
    pub fn assert_failure_contains(&self, expected: &str) {
        self.assert_failure();
        let err = self.error().unwrap_or("");
        if !err.contains(expected) {
            panic!("Expected error containing '{}' but got: {}", expected, err);
        }
    }

    /// Assert that logs contain the given string
    pub fn assert_logs_contain(&self, expected: &str) {
        let logs = self.logs();
        if !logs.iter().any(|log| log.contains(expected)) {
            panic!("Expected logs to contain '{}' but got: {:?}", expected, logs);
        }
    }

    /// Assert that gas used is less than the given amount
    pub fn assert_gas_lt(&self, limit: NearGas) {
        if self.total_gas_used.as_gas() >= limit.as_gas() {
            panic!("Expected gas < {} but used {}", limit, self.total_gas_used);
        }
    }
}

/// Mock response for external contracts
#[derive(Debug, Clone)]
pub enum MockResponse {
    /// Successful execution with return value
    Success(Vec<u8>),
    /// Execution failed with error message
    Failure(String),
    /// Contract panicked
    Panic(String),
}

impl MockResponse {
    /// Create a successful response with JSON value
    pub fn success_json<T: serde::Serialize>(value: &T) -> Self {
        MockResponse::Success(serde_json::to_vec(value).unwrap())
    }

    /// Create a successful response with raw bytes
    pub fn success(data: impl Into<Vec<u8>>) -> Self {
        MockResponse::Success(data.into())
    }

    /// Create a failure response
    pub fn failure(message: impl Into<String>) -> Self {
        MockResponse::Failure(message.into())
    }

    /// Create a panic response
    pub fn panic(message: impl Into<String>) -> Self {
        MockResponse::Panic(message.into())
    }
}
