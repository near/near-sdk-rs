use crate::multi_token::token::ClearedApproval;
use crate::multi_token::token::TokenId;
use near_sdk::json_types::U128;
use near_sdk::AccountId;
use near_sdk::ext_contract;

/// `resolve_transfer` will be called after `on_transfer`
#[ext_contract(ext_mt_resolver)]
pub trait MultiTokenResolver {
    /// Finalizes chain of cross-contract calls that started from `mt_transfer_call`
    ///
    /// Flow:
    ///
    /// 1. Sender calls `mt_transfer_call` on MT contract
    /// 2. MT contract transfers tokens from sender to receiver
    /// 3. MT contract calls `on_transfer` on receiver contract
    /// 4+. [receiver may make cross-contract calls]
    /// N. MT contract resolves chain with `mt_resolve_transfer` and may do anything
    ///
    /// Requirements:
    /// * Contract MUST forbid calls to this function by any account except self
    /// * If promise chain failed, contract MUST revert tokens transfer
    /// * If promise chain resolves with `true`, contract MUST return tokens to
    ///   `previous_owner_ids`
    ///
    /// Arguments:
    /// * `previous_owner_ids`: the owner prior to the call to `transfer_call`
    /// * `receiver_id`: the `receiver_id` argument given to `transfer_call`
    /// * `token_ids`: the vector of `token_id` argument given to `transfer_call`
    /// * `approvals`: if using Approval Management, contract MUST provide
    ///   set of original approved accounts in this argument, and restore these
    ///   approved accounts in case of revert.
    ///
    /// Returns total amount spent by the `receiver_id`, corresponding to the `token_id`.
    ///
    /// Example: if sender calls `transfer_call({ "amounts": ["100"], token_ids: ["55"], receiver_id: "games" })`,
    /// but `receiver_id` only uses 80, `on_transfer` will resolve with `["20"]`, and `resolve_transfer`
    /// will return `[80]`.

    fn mt_resolve_transfer(
        &mut self,
        previous_owner_ids: Vec<AccountId>,
        receiver_id: AccountId,
        token_ids: Vec<TokenId>,
        amounts: Vec<U128>,
        approvals: Option<Vec<Option<Vec<ClearedApproval>>>>,
    ) -> Vec<U128>;
}
