//! Utility functions for the multi-token standard.

use crate::multi_token::token::{Approval, TokenId};
use near_sdk::json_types::U128;
use near_sdk::{env, require, AccountId, NearToken, Promise};
use std::collections::HashMap;
use std::mem::size_of;

/// Calculate the storage bytes required for an approval entry.
/// This is used to determine storage costs when adding approvals.
pub fn bytes_for_approved_account_id(account_id: &AccountId) -> u64 {
    // The extra 4 bytes are from Borsh serialization to store the string length.
    // We also need to store the Approval struct (approval_id: u64, amount: u128).
    account_id.as_str().len() as u64
        + 4
        + size_of::<u64>() as u64  // approval_id
        + size_of::<u128>() as u64 // amount
}

/// Refund storage deposit for cleared approvals to the specified account.
/// Returns a Promise for the refund transfer, allowing the caller to chain or detach.
pub fn refund_approved_account_ids_iter<'a, I>(
    account_id: AccountId,
    approved_account_ids: I,
) -> Promise
where
    I: Iterator<Item = &'a AccountId>,
{
    let storage_released: u64 = approved_account_ids.map(bytes_for_approved_account_id).sum();
    Promise::new(account_id)
        .transfer(env::storage_byte_cost().saturating_mul(storage_released.into()))
}

/// Refund storage deposit for cleared approvals to the specified account.
/// Returns a Promise for the refund transfer, allowing the caller to chain or detach.
pub fn refund_approved_account_ids(
    account_id: AccountId,
    approved_account_ids: &HashMap<AccountId, Approval>,
) -> Promise {
    refund_approved_account_ids_iter(account_id, approved_account_ids.keys())
}

/// Refund excess deposit after storage is paid.
/// Panics if attached deposit is insufficient.
pub fn refund_deposit_to_account(storage_used: u64, account_id: AccountId) {
    let required_cost = env::storage_byte_cost().saturating_mul(storage_used.into());
    let attached_deposit = env::attached_deposit();

    require!(
        required_cost <= attached_deposit,
        format!("Must attach {} to cover storage", required_cost.exact_amount_display())
    );

    let refund = attached_deposit.saturating_sub(required_cost);
    if refund.as_yoctonear() > 1 {
        Promise::new(account_id).transfer(refund).detach();
    }
}

/// Refund excess deposit to the predecessor account.
pub fn refund_deposit(storage_used: u64) {
    refund_deposit_to_account(storage_used, env::predecessor_account_id())
}

/// Assert that at least 1 yoctoNEAR was attached for security.
pub fn assert_at_least_one_yocto() {
    require!(
        env::attached_deposit() >= NearToken::from_yoctonear(1),
        "Requires attached deposit of at least 1 yoctoNEAR"
    )
}

/// Validate that token_ids and amounts have the same length.
/// Panics if lengths don't match.
pub fn assert_valid_batch_args(token_ids: &[TokenId], amounts: &[U128]) {
    require!(token_ids.len() == amounts.len(), "token_ids and amounts must have the same length");
    require!(!token_ids.is_empty(), "token_ids cannot be empty");
}

/// Validate that approvals array matches token_ids length if provided.
/// Panics if lengths don't match.
pub fn assert_valid_batch_approvals(
    token_ids: &[TokenId],
    approvals: &Option<Vec<Option<(AccountId, u64)>>>,
) {
    if let Some(approvals) = approvals {
        require!(
            token_ids.len() == approvals.len(),
            "approvals must have the same length as token_ids"
        );
    }
}

/// Validate all batch arguments together.
pub fn assert_valid_batch_all(
    token_ids: &[TokenId],
    amounts: &[U128],
    approvals: &Option<Vec<Option<(AccountId, u64)>>>,
) {
    assert_valid_batch_args(token_ids, amounts);
    assert_valid_batch_approvals(token_ids, approvals);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bytes_for_approved_account_id() {
        let account_id: AccountId = "alice.near".parse().unwrap();
        let bytes = bytes_for_approved_account_id(&account_id);
        // 10 (account name) + 4 (string length) + 8 (u64) + 16 (u128) = 38
        assert_eq!(bytes, 38);
    }

    #[test]
    fn test_assert_valid_batch_args_valid() {
        let token_ids = vec!["1".to_string(), "2".to_string()];
        let amounts = vec![U128(100), U128(200)];
        assert_valid_batch_args(&token_ids, &amounts); // Should not panic
    }

    #[test]
    #[should_panic(expected = "token_ids and amounts must have the same length")]
    fn test_assert_valid_batch_args_mismatched() {
        let token_ids = vec!["1".to_string(), "2".to_string()];
        let amounts = vec![U128(100)];
        assert_valid_batch_args(&token_ids, &amounts);
    }

    #[test]
    #[should_panic(expected = "token_ids cannot be empty")]
    fn test_assert_valid_batch_args_empty() {
        let token_ids: Vec<TokenId> = vec![];
        let amounts: Vec<U128> = vec![];
        assert_valid_batch_args(&token_ids, &amounts);
    }

    #[test]
    fn test_assert_valid_batch_approvals_none() {
        let token_ids = vec!["1".to_string()];
        assert_valid_batch_approvals(&token_ids, &None); // Should not panic
    }

    #[test]
    fn test_assert_valid_batch_approvals_valid() {
        let token_ids = vec!["1".to_string(), "2".to_string()];
        let approvals = Some(vec![None, Some(("alice.near".parse().unwrap(), 1))]);
        assert_valid_batch_approvals(&token_ids, &approvals); // Should not panic
    }

    #[test]
    #[should_panic(expected = "approvals must have the same length as token_ids")]
    fn test_assert_valid_batch_approvals_mismatched() {
        let token_ids = vec!["1".to_string(), "2".to_string()];
        let approvals = Some(vec![None]);
        assert_valid_batch_approvals(&token_ids, &approvals);
    }
}
