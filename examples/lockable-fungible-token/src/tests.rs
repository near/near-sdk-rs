use near_sdk::test_utils::VMContextBuilder;
use near_sdk::test_utils::test_env::{alice, bob, carol};
use near_sdk::{VMContext, testing_env};

use super::*;

fn get_context(predecessor_account_id: AccountId) -> VMContext {
    VMContextBuilder::new().predecessor_account_id(predecessor_account_id).build()
}

#[test]
fn test_new() {
    let context = get_context(carol());
    testing_env!(context);
    let total_supply = 1_000_000_000_000_000u128;
    let contract = FunToken::new(bob(), U128(total_supply)).unwrap();
    assert_eq!(contract.get_total_supply(), U128(total_supply));
    assert_eq!(contract.get_unlocked_balance(bob()), U128(total_supply));
    assert_eq!(contract.get_total_balance(bob()), U128(total_supply));
}

#[test]
fn test_transfer() {
    let context = get_context(carol());
    testing_env!(context);
    let total_supply = 1_000_000_000_000_000u128;
    let mut contract = FunToken::new(carol(), U128(total_supply)).unwrap();
    let transfer_amount = total_supply / 3;
    contract.transfer(bob(), U128(transfer_amount)).unwrap();
    assert_eq!(contract.get_unlocked_balance(carol()), U128(total_supply - transfer_amount));
    assert_eq!(contract.get_unlocked_balance(bob()), U128(transfer_amount));
}

#[test]
fn test_lock_fail() {
    let context = get_context(carol());
    testing_env!(context);
    let total_supply = 1_000_000_000_000_000u128;
    let mut contract = FunToken::new(carol(), U128(total_supply)).unwrap();
    let transfer_amount = total_supply / 3;
    contract.lock(bob(), U128(transfer_amount)).unwrap_err();
}

#[test]
fn test_self_allowance_fail() {
    let context = get_context(carol());
    testing_env!(context);
    let total_supply = 1_000_000_000_000_000u128;
    let mut contract = FunToken::new(carol(), U128(total_supply)).unwrap();
    contract.set_allowance(carol(), U128(total_supply / 2)).unwrap_err();
}

#[test]
fn test_lock_and_unlock_owner() {
    let context = get_context(carol());
    testing_env!(context);
    let total_supply = 1_000_000_000_000_000u128;
    let mut contract = FunToken::new(carol(), U128(total_supply)).unwrap();
    assert_eq!(contract.get_total_supply(), U128(total_supply));
    let lock_amount = total_supply / 3;
    contract.lock(carol(), U128(lock_amount)).unwrap();
    assert_eq!(contract.get_unlocked_balance(carol()), U128(total_supply - lock_amount));
    assert_eq!(contract.get_total_balance(carol()), U128(total_supply));
    contract.unlock(carol(), U128(lock_amount)).unwrap();
    assert_eq!(contract.get_unlocked_balance(carol()), U128(total_supply));
    assert_eq!(contract.get_total_balance(carol()), U128(total_supply));
}

#[test]
fn test_lock_and_transfer() {
    let context = get_context(carol());
    testing_env!(context);
    let total_supply = 1_000_000_000_000_000u128;
    let mut contract = FunToken::new(carol(), U128(total_supply)).unwrap();
    assert_eq!(contract.get_total_supply(), U128(total_supply));
    let lock_amount = total_supply / 3;
    let transfer_amount = lock_amount / 3;
    // Locking
    contract.lock(carol(), U128(lock_amount)).unwrap();
    assert_eq!(contract.get_unlocked_balance(carol()), U128(total_supply - lock_amount));
    assert_eq!(contract.get_total_balance(carol()), U128(total_supply));
    for i in 1..=5 {
        // Transfer to bob
        contract.transfer(bob(), U128(transfer_amount)).unwrap();
        assert_eq!(
            contract.get_unlocked_balance(carol()),
            U128(std::cmp::min(total_supply - lock_amount, total_supply - transfer_amount * i))
        );
        assert_eq!(contract.get_total_balance(carol()), U128(total_supply - transfer_amount * i));
        assert_eq!(contract.get_unlocked_balance(bob()), U128(transfer_amount * i));
    }
}

#[test]
fn test_carol_escrows_to_bob_transfers_to_alice() {
    // Acting as carol
    testing_env!(get_context(carol()));
    let total_supply = 1_000_000_000_000_000u128;
    let mut contract = FunToken::new(carol(), U128(total_supply)).unwrap();
    assert_eq!(contract.get_total_supply(), U128(total_supply));
    let allowance = total_supply / 3;
    let transfer_amount = allowance / 3;
    contract.set_allowance(bob(), U128(allowance)).unwrap();
    assert_eq!(contract.get_allowance(carol(), bob()), U128(allowance));
    // Acting as bob now
    testing_env!(get_context(bob()));
    contract.transfer_from(carol(), alice(), U128(transfer_amount)).unwrap();
    assert_eq!(contract.get_total_balance(carol()), U128(total_supply - transfer_amount));
    assert_eq!(contract.get_unlocked_balance(alice()), U128(transfer_amount));
    assert_eq!(contract.get_allowance(carol(), bob()), U128(allowance - transfer_amount));
}

#[test]
fn test_carol_escrows_to_bob_locks_and_transfers_to_alice() {
    // Acting as carol
    testing_env!(get_context(carol()));
    let total_supply = 1_000_000_000_000_000u128;
    let mut contract = FunToken::new(carol(), U128(total_supply)).unwrap();
    assert_eq!(contract.get_total_supply(), U128(total_supply));
    let allowance = total_supply / 3;
    let transfer_amount = allowance / 3;
    let lock_amount = transfer_amount;
    contract.set_allowance(bob(), U128(allowance)).unwrap();
    assert_eq!(contract.get_allowance(carol(), bob()), U128(allowance));
    // Acting as bob now
    testing_env!(get_context(bob()));
    contract.lock(carol(), U128(lock_amount)).unwrap();
    assert_eq!(contract.get_allowance(carol(), bob()), U128(allowance - lock_amount));
    assert_eq!(contract.get_unlocked_balance(carol()), U128(total_supply - lock_amount));
    assert_eq!(contract.get_total_balance(carol()), U128(total_supply));
    contract.transfer_from(carol(), alice(), U128(transfer_amount)).unwrap();
    assert_eq!(contract.get_unlocked_balance(carol()), U128(total_supply - transfer_amount));
    assert_eq!(contract.get_unlocked_balance(alice()), U128(transfer_amount));
    assert_eq!(contract.get_allowance(carol(), bob()), U128(allowance - transfer_amount));
}

#[test]
fn test_lock_and_unlock_through_allowance() {
    // Acting as carol
    testing_env!(get_context(carol()));
    let total_supply = 1_000_000_000_000_000u128;
    let mut contract = FunToken::new(carol(), U128(total_supply)).unwrap();
    assert_eq!(contract.get_total_supply(), U128(total_supply));
    let allowance = total_supply / 3;
    let lock_amount = allowance / 2;
    contract.set_allowance(bob(), U128(allowance)).unwrap();
    assert_eq!(contract.get_allowance(carol(), bob()), U128(allowance));
    // Acting as bob now
    testing_env!(get_context(bob()));
    contract.lock(carol(), U128(lock_amount)).unwrap();
    assert_eq!(contract.get_allowance(carol(), bob()), U128(allowance - lock_amount));
    assert_eq!(contract.get_unlocked_balance(carol()), U128(total_supply - lock_amount));
    assert_eq!(contract.get_total_balance(carol()), U128(total_supply));
    contract.unlock(carol(), U128(lock_amount)).unwrap();
    assert_eq!(contract.get_allowance(carol(), bob()), U128(allowance));
    assert_eq!(contract.get_unlocked_balance(carol()), U128(total_supply));
    assert_eq!(contract.get_total_balance(carol()), U128(total_supply));
}

#[test]
fn test_set_allowance_during_lock() {
    // Acting as carol
    testing_env!(get_context(carol()));
    let total_supply = 1_000_000_000_000_000u128;
    let mut contract = FunToken::new(carol(), U128(total_supply)).unwrap();
    assert_eq!(contract.get_total_supply(), U128(total_supply));
    let allowance = 2 * total_supply / 3;
    let lock_amount = allowance / 2;
    contract.set_allowance(bob(), U128(allowance)).unwrap();
    assert_eq!(contract.get_allowance(carol(), bob()), U128(allowance));
    // Acting as bob now
    testing_env!(get_context(bob()));
    contract.lock(carol(), U128(lock_amount)).unwrap();
    assert_eq!(contract.get_allowance(carol(), bob()), U128(allowance - lock_amount));
    assert_eq!(contract.get_unlocked_balance(carol()), U128(total_supply - lock_amount));
    assert_eq!(contract.get_total_balance(carol()), U128(total_supply));
    // Acting as carol now
    testing_env!(get_context(carol()));
    contract.set_allowance(bob(), U128(allowance)).unwrap();
    assert_eq!(contract.get_allowance(carol(), bob()), U128(allowance - lock_amount));
}

#[test]
fn test_competing_locks() {
    // Acting as carol
    testing_env!(get_context(carol()));
    let total_supply = 1_000_000_000_000_000u128;
    let mut contract = FunToken::new(carol(), U128(total_supply)).unwrap();
    assert_eq!(contract.get_total_supply(), U128(total_supply));
    let allowance = 2 * total_supply / 3;
    let lock_amount = allowance;
    contract.set_allowance(bob(), U128(allowance)).unwrap();
    contract.set_allowance(alice(), U128(allowance)).unwrap();
    assert_eq!(contract.get_allowance(carol(), bob()), U128(allowance));
    assert_eq!(contract.get_allowance(carol(), alice()), U128(allowance));
    // Acting as bob now
    testing_env!(get_context(bob()));
    contract.lock(carol(), U128(lock_amount)).unwrap();
    assert_eq!(contract.get_allowance(carol(), bob()), U128(allowance - lock_amount));
    assert_eq!(contract.get_unlocked_balance(carol()), U128(total_supply - lock_amount));
    assert_eq!(contract.get_total_balance(carol()), U128(total_supply));
    // Acting as alice now
    testing_env!(get_context(alice()));
    contract.lock(carol(), U128(lock_amount)).unwrap_err();
}
