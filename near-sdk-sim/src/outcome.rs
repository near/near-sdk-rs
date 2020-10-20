use crate::transaction::ExecutionOutcome;
use near_primitives::transaction::ExecutionStatus::{SuccessReceiptId, SuccessValue};
use near_sdk::borsh::BorshDeserialize;
use near_sdk::serde_json::{json, Value};
use std::io;
use std::io::{Error, ErrorKind};

pub fn get_json_value(outcome: ExecutionOutcome) -> Value {
    use crate::transaction::ExecutionStatus::*;
    match outcome.status {
        SuccessValue(s) => near_sdk::serde_json::from_slice(&s).unwrap(),
        _ => json!({}),
    }
}

pub fn get_borsh_value<T: BorshDeserialize>(outcome: ExecutionOutcome) -> io::Result<T> {
    use crate::transaction::ExecutionStatus::*;
    match outcome.status {
        SuccessValue(s) => {
            let res = BorshDeserialize::try_from_slice(&s);
            res
        }
        _ => std::result::Result::Err(Error::new(ErrorKind::Other, "Not a success")),
    }
}

pub fn is_success(outcome: &ExecutionOutcome) -> bool {
    match outcome.status {
        SuccessValue(_) => true,
        SuccessReceiptId(_) => true,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use near_primitives::transaction::ExecutionStatus::SuccessValue;

    // #[derive(Serialize, Deserialize)]
    // struct Foo {
    //     id: String,
    // }

    #[test]
    fn value_test() {
        let value = json!({
          "id": "hello"
        });
        let status = SuccessValue(value.clone().to_string().as_bytes().to_vec());
        let mut outcome = ExecutionOutcome::default();
        outcome.status = status;
        assert_eq!(value, get_value(outcome));
    }
}
