use near_contract_standards::fungible_token::Balance;
use near_sdk::test_utils::{VMContextBuilder, accounts};
use near_sdk::testing_env;

use super::*;

const TOTAL_SUPPLY: Balance = 1_000_000_000_000_000;

fn get_context(predecessor_account_id: AccountId) -> VMContextBuilder {
    let mut builder = VMContextBuilder::new();
    builder
        .current_account_id(accounts(0))
        .signer_account_id(predecessor_account_id.clone())
        .predecessor_account_id(predecessor_account_id);
    builder
}

#[test]
fn test_new() {
    let mut context = get_context(accounts(1));
    testing_env!(context.build());
    let contract = Contract::new_default_meta(accounts(1).into(), TOTAL_SUPPLY.into());
    testing_env!(context.is_view(true).build());
    assert_eq!(contract.ft_total_supply().0, TOTAL_SUPPLY);
    assert_eq!(contract.ft_balance_of(accounts(1)).0, TOTAL_SUPPLY);
}

#[test]
#[should_panic(expected = "The contract is not initialized")]
fn test_default() {
    let context = get_context(accounts(1));
    testing_env!(context.build());
    let _contract = Contract::default();
}

#[test]
fn test_transfer() {
    let mut context = get_context(accounts(2));
    testing_env!(context.build());
    let mut contract = Contract::new_default_meta(accounts(2).into(), TOTAL_SUPPLY.into());
    testing_env!(
        context
            .storage_usage(env::storage_usage())
            .attached_deposit(contract.storage_balance_bounds().min.into())
            .predecessor_account_id(accounts(1))
            .build()
    );
    // Paying for account registration, aka storage deposit
    contract.storage_deposit(None, None);

    testing_env!(
        context
            .storage_usage(env::storage_usage())
            .attached_deposit(NearToken::from_yoctonear(1))
            .predecessor_account_id(accounts(2))
            .build()
    );
    let transfer_amount = TOTAL_SUPPLY / 3;
    contract.ft_transfer(accounts(1), transfer_amount.into(), None);

    testing_env!(
        context
            .storage_usage(env::storage_usage())
            .account_balance(env::account_balance())
            .is_view(true)
            .attached_deposit(NearToken::from_near(0))
            .build()
    );
    assert_eq!(contract.ft_balance_of(accounts(2)).0, (TOTAL_SUPPLY - transfer_amount));
    assert_eq!(contract.ft_balance_of(accounts(1)).0, transfer_amount);
}
