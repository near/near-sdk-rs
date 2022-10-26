use crate::multi_token::token::TokenId;
use near_sdk::AccountId;

/// Approval receiver is the trait for the method called (or attempted to be called) when an MT contract adds an approval for an account.
pub trait MultiTokenApprovalReceiver {
    /// Respond to notification that contract has been granted approval for a token.
    fn mt_on_approve(
        &mut self,
        tokens: Vec<TokenId>,
        owner_id: AccountId,
        approval_ids: Vec<u64>,
        msg: String,
    ) -> near_sdk::PromiseOrValue<String>;
}
