use crate::transaction::ExecutionOutcome;
use near_sdk::serde_json::json;

pub fn get_value(outcome: ExecutionOutcome) -> serde_json::Value {
    use crate::transaction::ExecutionStatus::*;
    match outcome.status {
        SuccessValue(s) => near_sdk::serde_json::from_slice(&s).unwrap(),
        _ => json!({}),
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
