use super::*;
use near_sdk::test_utils::{VMContextBuilder, accounts};
use near_sdk::{AccountId, testing_env};

fn get_context(predecessor_account_id: AccountId) -> near_sdk::VMContext {
    VMContextBuilder::new()
        .current_account_id(accounts(0))
        .signer_account_id(predecessor_account_id.clone())
        .predecessor_account_id(predecessor_account_id)
        .build()
}

#[test]
fn test_deploy_global_contract() {
    let context = get_context(accounts(1));
    testing_env!(context);

    let mut contract = GlobalFactoryContract::default();
    let code = vec![0u8; 100]; // Mock bytecode

    contract.deploy_global_contract("test_contract".to_string(), code.clone().into()).detach();

    // Check that the contract was recorded
    let stored_hash = contract.get_global_contract_hash("test_contract".to_string());
    assert!(stored_hash.is_some());

    let expected_hash = near_sdk::env::sha256_array(&code);
    assert_eq!(stored_hash.unwrap(), expected_hash);
}

#[test]
fn test_list_global_contracts() {
    let context = get_context(accounts(1));
    testing_env!(context);

    let mut contract = GlobalFactoryContract::default();
    let code = vec![0u8; 100];

    contract.deploy_global_contract("test_contract".to_string(), code.clone().into()).detach();

    let contracts = contract.get_global_contracts_registered_by_code_hash();

    let expected_hash: Base58CryptoHash =
        "EoFMvgbdQttJ3vLsVBcgZaWbhEGrJnpqda85qtbu7LbL".parse().unwrap();

    assert_eq!(contracts.len(), 1);
    assert_eq!(contracts[0].0, "test_contract");
    assert_eq!(contracts[0].1, expected_hash);
}
