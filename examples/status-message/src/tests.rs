use near_sdk::test_utils::{VMContextBuilder, get_logs};
use near_sdk::{VMContext, testing_env};

use super::*;

fn get_context(is_view: bool) -> VMContext {
    VMContextBuilder::new().signer_account_id("bob_near".parse().unwrap()).is_view(is_view).build()
}

#[test]
fn set_get_message() {
    let context = get_context(false);
    testing_env!(context);
    let mut contract = StatusMessage::default();
    contract.set_status("hello".to_string());
    // Flush the pending changes to avoid panic in the view method below due to the pending non-committed changes to the `store::LookupMap`:
    // HostError(ProhibitedInView { method_name: "storage_write" })
    contract.records.flush();
    assert_eq!(get_logs(), vec!["bob_near set_status with message hello"]);

    let context = get_context(true);
    testing_env!(context);
    assert_eq!("hello", contract.get_status("bob_near".parse().unwrap()).unwrap());
    assert_eq!(get_logs(), vec!["get_status for account_id bob_near"]);
}

#[test]
fn get_nonexistent_message() {
    let context = get_context(true);
    testing_env!(context);
    let contract = StatusMessage::default();
    assert_eq!(None, contract.get_status("francis.near".parse().unwrap()));
    assert_eq!(get_logs(), vec!["get_status for account_id francis.near"])
}
