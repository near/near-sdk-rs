uuse near_sdk::{test_utils::{testing_env, VMContextBuilder}, AccountId, PromiseResult};
use near_sdk::env;

// Let's say we have a contract method that does something based on a callback.
// This example will test it using the `promise_results` feature in `testing_env!`.

#[test]
fn test_contract_callback_with_successful_promise() {
    // Prepare mock promise results (Successful callback)
    let promise_results = vec![PromiseResult::Successful(b"mocked callback data".to_vec())];

    // Create the context and set the promise results
    let context = VMContextBuilder::new()
        .predecessor_account_id(AccountId::new_unchecked("alice.near".to_string()))
        .promise_results(promise_results)
        .build();

    // Initialize the environment with the context (this prepares the contract for testing)
    testing_env!(context);

    // Assume we have a contract with the following method:
    // fn process_callback(&self) -> String {
    //     let result = env::promise_result(0); // This would return the result of the first promise
    //     match result {
    //         PromiseResult::Successful(data) => String::from_utf8(data).unwrap(),
    //         _ => "Failed".to_string(),
    //     }
    // }

    // Simulating contract logic that calls process_callback() would return "mocked callback data"
    let result = process_callback();
    assert_eq!(result, "mocked callback data".to_string());
}

#[test]
fn test_contract_callback_with_failed_promise() {
    // Prepare mock promise results (Failed callback)
    let promise_results = vec![PromiseResult::Failed];

    // Create the context with failed promise result
    let context = VMContextBuilder::new()
        .predecessor_account_id(AccountId::new_unchecked("alice.near".to_string()))
        .promise_results(promise_results)
        .build();

    // Initialize the environment
    testing_env!(context);

    // Assume `process_callback` would now return "Failed" because the promise failed
    let result = process_callback();
    assert_eq!(result, "Failed".to_string());
}

// Sample contract function that will process the promise results
fn process_callback() -> String {
    let result = env::promise_result(0); // Assuming we're checking the first promise result
    match result {
        PromiseResult::Successful(data) => String::from_utf8(data).unwrap(),
        _ => "Failed".to_string(),
    }
}
