mod approval_impl;
mod receiver;

pub use approval_impl::*;
pub use receiver::*;

use crate::multi_token::token::TokenId;
use near_sdk::{AccountId, Promise};

/// Trait used in approval management
/// Specs - https://github.com/shipsgold/NEPs/blob/master/specs/Standards/MultiToken/ApprovalManagement.md
pub trait MultiTokenApproval {
    /// Add an approved account for a specific token.
    fn mt_approve(
        &mut self,
        token_id: TokenId,
        account_id: AccountId,
        msg: Option<String>,
    ) -> Option<Promise>;

    /// Revoke approvals granted to a specific user for a specific token.
    fn mt_revoke(&mut self, token_id: TokenId, account_id: AccountId);

    /// Revoke all approvals for a token
    fn mt_revoke_all(&mut self, token_id: TokenId);

    /// Check if account have access to transfer tokens
    fn mt_is_approved(
        &self,
        token_id: TokenId,
        approved_account_id: AccountId,
        approval_id: Option<u64>,
    ) -> bool;
}
