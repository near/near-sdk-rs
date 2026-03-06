use crate::multi_token::token::{ClearedApproval, TokenId};
use near_sdk::ext_contract;
use near_sdk::json_types::U128;
use near_sdk::AccountId;

/// Used when a multi token is transferred using `mt_transfer_call` or `mt_batch_transfer_call`.
/// This is the method that's called after `mt_on_transfer`.
/// This trait is implemented on the MT contract.
///
/// # Examples
///
/// ```
/// use near_sdk::{PanicOnDefault, AccountId, near};
/// use near_sdk::json_types::U128;
/// use near_contract_standards::multi_token::{MultiTokenResolver, TokenId, ClearedApproval};
///
/// #[near(contract_state)]
/// #[derive(PanicOnDefault)]
/// pub struct Contract {
///    // tokens: MultiToken,
/// }
/// #[near]
/// impl MultiTokenResolver for Contract {
///     #[private]
///     fn mt_resolve_transfer(
///         &mut self,
///         previous_owner_ids: Vec<AccountId>,
///         receiver_id: AccountId,
///         token_ids: Vec<TokenId>,
///         amounts: Vec<U128>,
///         approvals: Option<Vec<Option<Vec<ClearedApproval>>>>,
///     ) -> Vec<U128> {
///         vec![] // Would need implementation
///     }
/// }
/// ```
///
#[ext_contract(ext_mt_resolver)]
pub trait MultiTokenResolver {
    /// Finalize an `mt_transfer_call` or `mt_batch_transfer_call` chain of cross-contract calls.
    ///
    /// The `mt_transfer_call` process:
    ///
    /// 1. Sender calls `mt_transfer_call` on MT contract
    /// 2. MT contract transfers token from sender to receiver
    /// 3. MT contract calls `mt_on_transfer` on receiver contract
    /// 4+. [receiver contract may make other cross-contract calls]
    /// N. MT contract resolves promise chain with `mt_resolve_transfer`, and may
    ///    transfer token back to sender
    ///
    /// Requirements:
    /// * Contract MUST forbid calls to this function by any account except self
    /// * If promise chain failed, contract MUST revert token transfer
    /// * If promise chain resolves with amounts to return, contract MUST return those
    ///   tokens to `previous_owner_ids`
    ///
    /// Arguments:
    /// * `previous_owner_ids`: the owners prior to the call to `mt_transfer_call`
    /// * `receiver_id`: the `receiver_id` argument given to `mt_transfer_call`
    /// * `token_ids`: the `token_ids` argument given to `mt_transfer_call`
    /// * `amounts`: the `amounts` argument given to `mt_transfer_call`
    /// * `approvals`: if using Approval Management, contract MUST provide
    ///   set of original approvals in this argument, and restore the
    ///   approved accounts in case of revert.
    ///
    /// Returns total amount spent by the `receiver_id`, corresponding to each `token_id`.
    fn mt_resolve_transfer(
        &mut self,
        previous_owner_ids: Vec<AccountId>,
        receiver_id: AccountId,
        token_ids: Vec<TokenId>,
        amounts: Vec<U128>,
        approvals: Option<Vec<Option<Vec<ClearedApproval>>>>,
    ) -> Vec<U128>;
}
