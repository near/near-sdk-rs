use crate::hash::CryptoHash;
use crate::runtime::RuntimeStandalone;
use crate::transaction::{ExecutionOutcome, ExecutionStatus};
use core::fmt;
use near_primitives::transaction::ExecutionStatus::{SuccessReceiptId, SuccessValue};
use near_sdk::borsh::BorshDeserialize;
use near_sdk::serde::export::Formatter;
use near_sdk::serde_json;
use near_sdk::serde_json::Value;
use std::cell::RefCell;
use std::fmt::Debug;
use std::io;
use std::io::{Error, ErrorKind};
use std::rc::Rc;

pub type TxResult = Result<ExecutionOutcome, ExecutionOutcome>;
pub type ViewResult = Result<(Vec<u8>, Vec<String>), Box<dyn std::error::Error>>;

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

    pub fn is_success(&self) -> bool {
        match &(self.outcome).status {
            SuccessValue(_) => true,
            SuccessReceiptId(_) => true,
            _ => false,
        }
    }

    pub fn assert_success(&self) {
        assert!(self.is_success(), "Outcome was a failure");
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
            Some(o) => !o.is_success(),
            _ => false,
        });
        res
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

#[cfg(test)]
mod tests {
    use super::*;
    use near_primitives::transaction::ExecutionStatus::SuccessValue;

    #[test]
    fn value_test() {
        let value = json!({
          "id": "hello"
        });
        let status = SuccessValue(value.clone().to_string().as_bytes().to_vec());
        let mut outcome = ExecutionOutcome::default();
        outcome.status = status;
        assert_eq!(value, get_json_value(outcome));
    }
}
