mod approval_impl;
mod receiver;

pub use approval_impl::*;
pub use receiver::*;

use crate::multi_token::token::TokenId;
use near_sdk::{AccountId, Balance, Promise};

/// Trait used in approval management
/// Specs - https://github.com/shipsgold/NEPs/blob/master/specs/Standards/MultiToken/ApprovalManagement.md
pub trait MultiTokenApproval {
    /// Add an approved account for a specific set of tokens
    fn mt_approve(
        &mut self,
        token_ids: Vec<TokenId>,
        amounts: Vec<Balance>,
        grantee_id: AccountId,
        msg: Option<String>,
    ) -> Option<Promise>;

    /// Revoke approvals granted to a specific user for a specific set of tokens.
    fn mt_revoke(&mut self, token_ids: Vec<TokenId>, account_id: AccountId);

    /// Revoke all approvals for a token
    fn mt_revoke_all(&mut self, token_ids: Vec<TokenId>);

    /// Check if account have access to transfer tokens
    fn mt_is_approved(
        &self,
        token_ids: Vec<TokenId>,
        approved_account_id: AccountId,
        amounts: Vec<Balance>,
        approval_ids: Option<Vec<u64>>,
    ) -> bool;
}
