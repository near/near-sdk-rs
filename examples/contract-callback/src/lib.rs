use near_sdk::{env};

pub fn process_callback() -> String {
    let result = env::promise_result(0);
    match result {
        near_sdk::PromiseResult::Successful(data) => String::from_utf8(data).unwrap(),
        _ => "Failed".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use near_sdk::{
        test_utils::{testing_env, VMContextBuilder, get_created_receipts},
        PromiseResult,
    };

    #[test]
    fn test_created_receipts() {
        let context = VMContextBuilder::new()
            .predecessor_account_id(AccountId::new_unchecked("alice.near".to_string()))
            .build();
        testing_env!(context);

        Promise::new(AccountId::new_unchecked("bob.near".to_string()))
            .transfer(1_000_000_000_000_000_000_000_000);

        let receipts = get_created_receipts();
        assert_eq!(receipts.len(), 1);
        let receipt = &receipts[0];
        assert_eq!(receipt.receiver_id.as_str(), "bob.near");

        assert!(receipt.actions.iter().any(|action| {
            matches!(action, near_sdk::env::ReceiptAction::Transfer { .. })
        }));
    }

    #[test]
    fn test_contract_callback_with_successful_promise() {
        let promise_results = vec![PromiseResult::Successful(b"mocked callback data".to_vec())];

        let context = VMContextBuilder::new()
            .predecessor_account_id(AccountId::new_unchecked("alice.near".to_string()))
            .promise_results(promise_results)
            .build();

        testing_env!(context);

        let result = process_callback();
        assert_eq!(result, "mocked callback data".to_string());
    }

    #[test]
    fn test_contract_callback_with_failed_promise() {
        let promise_results = vec![PromiseResult::Failed];

        let context = VMContextBuilder::new()
            .predecessor_account_id(AccountId::new_unchecked("alice.near".to_string()))
            .promise_results(promise_results)
            .build();

        testing_env!(context);

        let result = process_callback();
        assert_eq!(result, "Failed".to_string());
    }
}
