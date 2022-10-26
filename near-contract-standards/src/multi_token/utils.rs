use std::{fmt::Display, mem::size_of};

use crate::multi_token::token::Approval;
use near_sdk::json_types::U128;
use near_sdk::{env, require, AccountId, Balance, CryptoHash, Promise};
use std::collections::HashMap;

pub fn hash_account_id(account_id: &AccountId) -> CryptoHash {
    let mut hash = CryptoHash::default();
    hash.copy_from_slice(&env::sha256(account_id.as_bytes()));
    hash
}

pub fn refund_deposit_to_account(storage_used: u64, account_id: AccountId) {
    let required_cost = env::storage_byte_cost() * Balance::from(storage_used);
    let attached_deposit = env::attached_deposit();

    require!(
        required_cost <= attached_deposit,
        format!("Must attach {} yoctoNEAR to cover storage", required_cost)
    );

    let refund = attached_deposit - required_cost;
    if refund > 1 {
        Promise::new(account_id).transfer(refund);
    }
}

/// Assumes that the precedecessor will be refunded
pub fn refund_deposit(storage_used: u64) {
    refund_deposit_to_account(storage_used, env::predecessor_account_id())
}

// TODO: need a way for end users to determine how much an approval will cost.
pub fn bytes_for_approved_account_id(account_id: &AccountId) -> u64 {
    // The extra 4 bytes are coming from Borsh serialization to store the length of the string.
    account_id.as_str().len() as u64 + 4 + size_of::<u64>() as u64
}

// validate that an approval exists with matching approval_id and sufficient balance.
pub fn check_and_apply_approval(
    approvals_by_account_id: &mut HashMap<AccountId, HashMap<AccountId, Approval>>,
    account_id: &AccountId,
    sender_id: &AccountId,
    approval_id: &u64,
    amount: Balance,
) -> Vec<(AccountId, u64, U128)> {
    let by_sender_id =
        approvals_by_account_id.remove(account_id).unwrap_or_else(|| panic!("Unauthorized"));
    let stored_approval = by_sender_id.get(sender_id).unwrap_or_else(|| panic!("Unauthorized"));

    require!(
        stored_approval.approval_id.eq(approval_id) && stored_approval.amount.eq(&amount),
        "Unauthorized"
    );

    // Given that we are consuming the approval, remove all other approvals granted to that account for that token.
    // The user will need to generate fresh approvals as required.
    // Return the now-deleted approvals, so that caller may restore them in case of revert.
    by_sender_id
        .into_iter()
        .map(|(key, approval)| (key, approval.approval_id, U128(approval.amount)))
        .collect()
}

pub enum Entity {
    Contract,
    Token,
}

impl Display for Entity {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let as_string = match self {
            Entity::Contract => "contract",
            Entity::Token => "token",
        };
        write!(f, "{}", as_string)
    }
}

pub fn expect_approval<T>(o: Option<T>, entity: Entity) -> T {
    o.unwrap_or_else(|| panic!("Approval Management is not supported by {}", entity))
}

pub fn unauthorized_assert(account_id: &AccountId) {
    require!(account_id == &env::predecessor_account_id())
}
