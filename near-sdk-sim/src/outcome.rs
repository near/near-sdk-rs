use crate::hash::CryptoHash;
use crate::runtime::{init_runtime, RuntimeStandalone};
use crate::transaction::{ExecutionOutcome, ExecutionStatus};
use core::fmt;
use near_primitives::transaction::ExecutionStatus::{SuccessReceiptId, SuccessValue};
use near_sdk::borsh::BorshDeserialize;
use near_sdk::serde::de::DeserializeOwned;
use near_sdk::serde::export::Formatter;
use near_sdk::serde_json;
use near_sdk::serde_json::Value;
use std::borrow::Borrow;
use std::cell::RefCell;
use std::fmt::Debug;
use std::io;
use std::io::{Error, ErrorKind};
use std::rc::Rc;

pub type TxResult = Result<ExecutionOutcome, ExecutionOutcome>;

#[derive(Clone)]
pub struct ExecutionResult {
    runtime: Rc<RefCell<RuntimeStandalone>>,
    outcome: ExecutionOutcome,
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
        )
    }
}

impl ExecutionResult {
    pub fn new(outcome: ExecutionOutcome, runtime: &Rc<RefCell<RuntimeStandalone>>) -> Self {
        Self { runtime: Rc::clone(runtime), outcome }
    }

    pub fn get_json_value(&self) -> serde_json::Result<Value> {
        use crate::transaction::ExecutionStatus::*;
        match &(self.outcome).status {
            SuccessValue(s) => near_sdk::serde_json::from_slice(&s),
            err => panic!("Expected Success value but got: {:#?}", err),
        }
    }

    pub fn get_borsh_value<T: BorshDeserialize>(&self) -> io::Result<T> {
        use crate::transaction::ExecutionStatus::*;
        match &(self.outcome).status {
            SuccessValue(s) => {
                let res = BorshDeserialize::try_from_slice(&s);
                res
            }
            _ => std::result::Result::Err(Error::new(
                ErrorKind::Other,
                "Cannot get value of failed transaction",
            )),
        }
    }

    pub fn from_json_value<T: DeserializeOwned>(&self) -> Result<T, near_sdk::serde_json::Error> {
        near_sdk::serde_json::from_value(self.get_json_value()?)
    }

    pub fn is_ok(&self) -> bool {
        match &(self.outcome).status {
            SuccessValue(_) => true,
            SuccessReceiptId(_) => true,
            _ => false,
        }
    }

    pub fn has_value(&self) -> bool {
        match &(self.outcome).status {
            SuccessValue(_) => true,
            _ => false,
        }
    }

    pub fn assert_success(&self) {
        assert!(self.is_ok(), "Outcome was a failure");
    }

    fn get_outcome(&self, hash: &CryptoHash) -> Option<ExecutionResult> {
        match (*self.runtime).borrow().outcome(hash) {
            Some(out) => Some(ExecutionResult::new(out, &self.runtime)),
            None => None,
        }
    }

    pub fn get_receipt_outcomes(&self) -> Vec<Option<ExecutionResult>> {
        self.get_outcomes(&self.outcome.receipt_ids)
    }

    fn get_outcomes(&self, ids: &Vec<CryptoHash>) -> Vec<Option<ExecutionResult>> {
        ids.iter().map(|id| self.get_outcome(&id)).collect()
    }

    pub fn get_last_outcomes(&self) -> Vec<Option<ExecutionResult>> {
        self.get_outcomes(&(*self.runtime).borrow().last_outcomes)
    }

    pub fn find_errors(&self) -> Vec<Option<ExecutionResult>> {
        let mut res = self.get_last_outcomes();
        res.retain(|outcome| match outcome {
            Some(o) => !o.is_ok(),
            _ => false,
        });
        res
    }

    pub fn status(&self) -> ExecutionStatus {
        self.outcome.status.clone()
    }
}

pub fn outcome_into_result(
    outcome: ExecutionOutcome,
    runtime: &Rc<RefCell<RuntimeStandalone>>,
) -> ExecutionResult {
    match outcome.status {
        ExecutionStatus::SuccessValue(_) |
        ExecutionStatus::Failure(_) => ExecutionResult::new(outcome, runtime),
        ExecutionStatus::SuccessReceiptId(_) => panic!("Unresolved ExecutionOutcome run runtime.resolve(tx) to resolve the final outcome of tx"),
        ExecutionStatus::Unknown => unreachable!()
    }
}

#[derive(Debug)]
pub struct ViewResult {
    result: Result<Vec<u8>, Box<dyn std::error::Error>>,
    logs: Vec<String>,
}

impl ViewResult {
    pub fn new(result: Result<Vec<u8>, Box<dyn std::error::Error>>, logs: Vec<String>) -> Self {
        Self { result, logs }
    }

    pub fn logs(&self) -> &Vec<String> {
        &self.logs
    }

    pub fn is_err(&self) -> bool {
        self.result.is_err()
    }

    pub fn is_ok(&self) -> bool {
        self.result.is_ok()
    }

    pub fn unwrap(&self) -> Vec<u8> {
        (&self.result).as_ref().borrow().unwrap().clone()
    }

    pub fn unwrap_err(&self) -> &dyn std::error::Error {
        (&self.result).as_ref().borrow().unwrap_err().as_ref().borrow()
    }

    pub fn get_json_value(&self) -> near_sdk::serde_json::Result<Value> {
        near_sdk::serde_json::from_slice(&self.result.as_ref().unwrap())
    }

    pub fn from_borsh_value<T: BorshDeserialize>(&self) -> io::Result<T> {
        BorshDeserialize::try_from_slice(&self.result.as_ref().unwrap())
    }

    pub fn from_json_value<T: DeserializeOwned>(&self) -> Result<T, near_sdk::serde_json::Error> {
        near_sdk::serde_json::from_value(self.get_json_value()?)
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
        let status = SuccessValue(value.clone().to_string().as_bytes().to_vec());
        let mut outcome = ExecutionOutcome::default();
        outcome.status = status;
        let result = outcome_into_result(outcome, &Rc::new(RefCell::new(init_runtime(None).0)));
        assert_eq!(value, result.get_json_value().unwrap());
    }
}
