use std::{fmt::Display, mem::size_of};

use near_sdk::{env, require, AccountId, Balance, Promise};
use crate::multi_token::token::TokenId;

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

pub fn expect_approval_for_token<T>(o: Option<T>, token_id: &TokenId) -> T {
    o.unwrap_or_else(|| panic!("No approvals for token {}", token_id))
}

pub fn unauthorized_assert(account_id: &AccountId) {
    require!(account_id == &env::predecessor_account_id())
}
