use crate::hash::CryptoHash;
use crate::runtime::{init_runtime, RuntimeStandalone};
use crate::transaction::{ExecutionOutcome, ExecutionStatus};
use core::fmt;
use near_primitives::profile::ProfileData;
use near_primitives::transaction::ExecutionStatus::{SuccessReceiptId, SuccessValue};
use near_primitives::types::AccountId;
use near_sdk::borsh::BorshDeserialize;
use near_sdk::serde::de::DeserializeOwned;
use near_sdk::serde_json::Value;
use near_sdk::Gas;
use std::borrow::Borrow;
use std::cell::RefCell;
use std::fmt::Debug;
use std::fmt::Formatter;
use std::rc::Rc;

pub type TxResult = Result<ExecutionOutcome, ExecutionOutcome>;

/// An ExecutionResult is created by a UserAccount submitting a transaction.
/// It wraps an ExecutionOutcome which is the same object returned from an RPC call.
#[derive(Clone)]
pub struct ExecutionResult {
    runtime: Rc<RefCell<RuntimeStandalone>>,
    outcome: ExecutionOutcome,
    hash: CryptoHash,
}

impl Debug for ExecutionResult {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("ExecutionResult").field("outcome", &self.outcome).finish()
    }
}

impl Default for ExecutionResult {
    fn default() -> Self {
        ExecutionResult::new(
            ExecutionOutcome::default(),
            &Rc::new(RefCell::new(init_runtime(None).0)),
            CryptoHash::default(),
        )
    }
}

impl ExecutionResult {
    #[doc(hidden)]
    pub fn new(
        outcome: ExecutionOutcome,
        runtime: &Rc<RefCell<RuntimeStandalone>>,
        hash: CryptoHash,
    ) -> Self {
        Self { runtime: Rc::clone(runtime), outcome, hash }
    }

    /// Interpret the SuccessValue as a JSON value
    pub fn unwrap_json_value(&self) -> Value {
        use crate::transaction::ExecutionStatus::*;
        match &(self.outcome).status {
            SuccessValue(s) => near_sdk::serde_json::from_slice(s).unwrap(),
            err => panic!("Expected Success value but got: {:#?}", err),
        }
    }

    /// Deserialize SuccessValue from Borsh
    pub fn unwrap_borsh<T: BorshDeserialize>(&self) -> T {
        use crate::transaction::ExecutionStatus::*;
        match &(self.outcome).status {
            SuccessValue(s) => BorshDeserialize::try_from_slice(s).unwrap(),
            _ => panic!("Cannot get value of failed transaction"),
        }
    }

    /// Deserialize SuccessValue from JSON
    pub fn unwrap_json<T: DeserializeOwned>(&self) -> T {
        near_sdk::serde_json::from_value(self.unwrap_json_value()).unwrap()
    }

    /// Check if transaction was successful
    pub fn is_ok(&self) -> bool {
        matches!(&(self.outcome).status, SuccessValue(_) | SuccessReceiptId(_))
    }

    /// Test whether there is a SuccessValue
    pub fn has_value(&self) -> bool {
        matches!(self.outcome.status, SuccessValue(_))
    }

    /// Asserts that the outcome is successful
    pub fn assert_success(&self) {
        assert!(self.is_ok(), "Outcome {:#?} was a failure", self.outcome);
    }

    /// Lookup an execution result from a hash
    pub fn lookup_hash(&self, hash: &CryptoHash) -> Option<ExecutionResult> {
        self.get_outcome(hash)
    }

    fn get_outcome(&self, hash: &CryptoHash) -> Option<ExecutionResult> {
        (*self.runtime)
            .borrow()
            .outcome(hash)
            .map(|out| ExecutionResult::new(out, &self.runtime, *hash))
    }

    /// Reference to internal ExecutionOutcome
    pub fn outcome(&self) -> &ExecutionOutcome {
        &self.outcome
    }

    /// Return results of promises from the `receipt_ids` in the ExecutionOutcome
    pub fn get_receipt_results(&self) -> Vec<Option<ExecutionResult>> {
        self.get_outcomes(&self.outcome.receipt_ids)
    }

    fn get_outcomes(&self, ids: &[CryptoHash]) -> Vec<Option<ExecutionResult>> {
        ids.iter().map(|id| self.get_outcome(id)).collect()
    }

    /// Return the results of any promises created since the last transaction
    pub fn promise_results(&self) -> Vec<Option<ExecutionResult>> {
        self.get_outcomes(&(*self.runtime).borrow().last_outcomes)
    }

    pub fn promise_errors(&self) -> Vec<Option<ExecutionResult>> {
        let mut res = self.promise_results();
        res.retain(|outcome| match outcome {
            Some(o) => !o.is_ok(),
            _ => false,
        });
        res
    }

    /// Execution status. Contains the result in case of successful execution.
    /// NOTE: Should be the latest field since it contains unparsable by light client
    /// ExecutionStatus::Failure
    pub fn status(&self) -> ExecutionStatus {
        self.outcome.status.clone()
    }

    /// The amount of the gas burnt by the given transaction or receipt.
    pub fn gas_burnt(&self) -> Gas {
        Gas(self.outcome.gas_burnt)
    }

    /// The amount of tokens burnt corresponding to the burnt gas amount.
    /// This value doesn't always equal to the `gas_burnt` multiplied by the gas price, because
    /// the prepaid gas price might be lower than the actual gas price and it creates a deficit.
    pub fn tokens_burnt(&self) -> u128 {
        self.outcome.tokens_burnt
    }

    /// Logs from this transaction or receipt.
    pub fn logs(&self) -> &Vec<String> {
        &self.outcome.logs
    }

    /// The id of the account on which the execution happens. For transaction this is signer_id,
    /// for receipt this is receiver_id.
    pub fn executor_id(&self) -> &AccountId {
        &self.outcome.executor_id
    }

    /// Receipt IDs generated by this transaction or receipt.
    pub fn receipt_ids(&self) -> &Vec<CryptoHash> {
        &self.outcome.receipt_ids
    }

    pub fn profile_data(&self) -> ProfileData {
        (*self.runtime).borrow().profile_of_outcome(&self.hash).unwrap()
    }
}

#[doc(hidden)]
pub fn outcome_into_result(
    outcome: (CryptoHash, ExecutionOutcome),
    runtime: &Rc<RefCell<RuntimeStandalone>>,
) -> ExecutionResult {
    match (outcome.1).status {
        ExecutionStatus::SuccessValue(_) |
        ExecutionStatus::Failure(_) => ExecutionResult::new(outcome.1, runtime, outcome.0),
        ExecutionStatus::SuccessReceiptId(_) => panic!("Unresolved ExecutionOutcome run runtime.resolve(tx) to resolve the final outcome of tx"),
        ExecutionStatus::Unknown => unreachable!()
    }
}

/// The result of a view call.  Contains the logs made during the view method call and Result value,
/// which can be unwrapped and deserialized.
#[derive(Debug)]
pub struct ViewResult {
    result: Result<Vec<u8>, Box<dyn std::error::Error>>,
    logs: Vec<String>,
}

impl ViewResult {
    pub fn new(result: Result<Vec<u8>, Box<dyn std::error::Error>>, logs: Vec<String>) -> Self {
        Self { result, logs }
    }

    /// Logs made during the view call
    pub fn logs(&self) -> &Vec<String> {
        &self.logs
    }

    pub fn is_err(&self) -> bool {
        self.result.is_err()
    }

    pub fn is_ok(&self) -> bool {
        self.result.is_ok()
    }

    /// Attempt unwrap the value returned by the view call and panic if it is an error
    pub fn unwrap(&self) -> Vec<u8> {
        (&self.result).as_ref().borrow().unwrap().clone()
    }

    pub fn unwrap_err(&self) -> &dyn std::error::Error {
        (&self.result).as_ref().borrow().unwrap_err().as_ref().borrow()
    }

    /// Interpret the value as a JSON::Value
    pub fn unwrap_json_value(&self) -> Value {
        near_sdk::serde_json::from_slice(self.result.as_ref().expect("ViewResult is an error"))
            .unwrap()
    }

    /// Deserialize the value with Borsh
    pub fn unwrap_borsh<T: BorshDeserialize>(&self) -> T {
        BorshDeserialize::try_from_slice(self.result.as_ref().expect("ViewResult is an error"))
            .unwrap()
    }

    /// Deserialize the value with JSON
    pub fn unwrap_json<T: DeserializeOwned>(&self) -> T {
        near_sdk::serde_json::from_value(self.unwrap_json_value()).unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::init_runtime;
    use near_primitives::transaction::ExecutionStatus::SuccessValue;
    use near_sdk::serde_json::json;

    #[test]
    fn value_test() {
        let value = json!({
          "id": "hello"
        });
        let status = SuccessValue(value.to_string().as_bytes().to_vec());
        let outcome = ExecutionOutcome { status, ..Default::default() };
        let result = outcome_into_result(
            (CryptoHash::default(), outcome),
            &Rc::new(RefCell::new(init_runtime(None).0)),
        );
        assert_eq!(value, result.unwrap_json_value());
    }
}
