//! Approval management for the Multi Token standard (NEP-245).
//!
//! This module provides the `MultiTokenApproval` trait and its implementation
//! for the `MultiToken` struct, allowing token owners to approve other accounts
//! to transfer tokens on their behalf.

mod approval_impl;

use crate::multi_token::token::TokenId;
use near_sdk::json_types::U128;
use near_sdk::{ext_contract, AccountId, Promise};

/// Trait for approval management in multi-token contracts.
///
/// This allows token owners to approve other accounts (individuals or contracts)
/// to transfer specific tokens on their behalf, similar to ERC-1155's approval system.
#[ext_contract(ext_mt_approval)]
pub trait MultiTokenApproval {
    /// Approve an account to transfer tokens on behalf of the owner.
    ///
    /// # Requirements
    /// * Caller must attach at least 1 yoctoⓃ for security purposes
    /// * Contract MAY require larger deposit to cover storage costs
    /// * Contract MUST panic if called by someone other than token owner
    /// * Contract MUST increment approval ID even if re-approving an account
    /// * If `msg` is present, contract MUST call `mt_on_approve` on `account_id`
    ///
    /// # Arguments
    /// * `token_ids` - The tokens to approve
    /// * `amounts` - The amounts to approve for each token
    /// * `account_id` - The account to approve
    /// * `msg` - Optional message to pass to `mt_on_approve`
    ///
    /// # Returns
    /// If `msg` is given, returns the promise from `mt_on_approve`. Otherwise returns None.
    fn mt_approve(
        &mut self,
        token_ids: Vec<TokenId>,
        amounts: Vec<U128>,
        account_id: AccountId,
        msg: Option<String>,
    ) -> Option<Promise>;

    /// Revoke approval for a specific account on specific tokens.
    ///
    /// # Requirements
    /// * Caller must attach 1 yoctoⓃ for security purposes
    /// * Contract MUST panic if called by someone other than token owner
    ///
    /// # Arguments
    /// * `token_ids` - The tokens to revoke approval for
    /// * `account_id` - The account to revoke
    fn mt_revoke(&mut self, token_ids: Vec<TokenId>, account_id: AccountId);

    /// Revoke all approvals for specific tokens.
    ///
    /// # Requirements
    /// * Caller must attach 1 yoctoⓃ for security purposes
    /// * Contract MUST panic if called by someone other than token owner
    ///
    /// # Arguments
    /// * `token_ids` - The tokens to revoke all approvals for
    fn mt_revoke_all(&mut self, token_ids: Vec<TokenId>);

    /// Check if an account is approved to transfer specific tokens.
    ///
    /// # Arguments
    /// * `token_ids` - The tokens to check
    /// * `approved_account_id` - The account to check approval for
    /// * `amounts` - The amounts to check (must be approved for at least these amounts)
    /// * `approval_ids` - Optional approval IDs to verify
    ///
    /// # Returns
    /// `true` if the account is approved for all specified tokens with sufficient amounts
    fn mt_is_approved(
        &self,
        token_ids: Vec<TokenId>,
        approved_account_id: AccountId,
        amounts: Vec<U128>,
        approval_ids: Option<Vec<u64>>,
    ) -> bool;
}

/// Trait for contracts that want to be notified when they receive approval.
#[ext_contract(ext_mt_approval_receiver)]
pub trait MultiTokenApprovalReceiver {
    /// Called when this contract receives approval for tokens.
    ///
    /// # Arguments
    /// * `token_ids` - The tokens that were approved
    /// * `amounts` - The amounts approved
    /// * `owner_id` - The owner who granted the approval
    /// * `approval_ids` - The approval IDs for each token
    /// * `msg` - The message passed from `mt_approve`
    fn mt_on_approve(
        &mut self,
        token_ids: Vec<TokenId>,
        amounts: Vec<U128>,
        owner_id: AccountId,
        approval_ids: Vec<u64>,
        msg: String,
    );
}
