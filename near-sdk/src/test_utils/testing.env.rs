/// Sets up the testing environment for unit tests of smart contracts.
///
/// The `testing_env!` macro initializes the runtime with the provided context,
/// allowing you to simulate contract execution under various conditions.
/// This is especially useful for setting up the caller account, attached deposit,
/// block height, prepaid gas, promise results, and more.
///
/// # Example
///
/// ```
/// use near_sdk::test_utils::{get_created_receipts, VMContextBuilder};
/// use near_sdk::{testing_env, PromiseResult};
///
/// fn get_context() -> VMContextBuilder {
///     VMContextBuilder::new()
///         .current_account_id("contract.testnet".parse().unwrap())
///         .signer_account_id("alice.testnet".parse().unwrap())
///         .predecessor_account_id("bob.testnet".parse().unwrap())
///         .attached_deposit(1_000_000_000_000_000_000_000_000) // 1 NEAR
///         .block_index(42)
///         .block_timestamp(1_600_000_000_000_000_000)
///         .prepaid_gas(10u64.pow(18))
/// }
///
/// #[test]
/// fn test_contract_behavior_with_promise_results_and_receipts() {
///     let mut context = get_context();
///     testing_env!(context.build());
///
///     // let mut contract = Contract::new();
///
///     let promise_results = vec![PromiseResult::Successful(
///         b\"{\\\"message\\\": \\\"success\\\"}\".to_vec(),
///     )];
///     context = context.promise_results(promise_results);
///     testing_env!(context.build());
///
///     // contract.handle_callback();
///
///     let receipts = get_created_receipts();
///     assert!(!receipts.is_empty(), \"Expected at least one receipt to be created\");
///     println!(\"Created Receipts: {:?}\", receipts);
/// }
/// ```
///
/// # Note
/// - You can call `testing_env!` multiple times in a single test to simulate different
///   stages of contract execution, such as before and after callbacks.
/// - Use `get_created_receipts()` to inspect outgoing cross-contract calls.
/// - Use `.promise_results()` on the `VMContextBuilder` to simulate callback results.
///
/// # See also
/// - [`VMContextBuilder`](crate::test_utils::VMContextBuilder)
/// - [`get_created_receipts`](crate::test_utils::get_created_receipts)
/// - [`PromiseResult`](crate::PromiseResult)
#[macro_export]
macro_rules! testing_env {
    ($context:expr) => {
        $crate::env::set_blockchain_interface(Box::new($context.clone()));
    };
}
