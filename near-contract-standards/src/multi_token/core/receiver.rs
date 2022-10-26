use crate::multi_token::token::TokenId;
use near_sdk::json_types::U128;
use near_sdk::{AccountId, PromiseOrValue};

/// Used when an MT is transferred using `transfer_call`. This trait should be implemented on receiving contract
pub trait MultiTokenReceiver {
    /// Take some action after receiving a multi-token's
    ///
    /// ## Requirements:
    /// * Contract MUST restrict calls to this function to a set of whitelisted MT
    ///   contracts
    /// * Contract MUST panic if `token_ids` length does not equal `amounts`
    ///   length
    /// * Contract MUST panic if `previous_owner_ids` length does not equal `token_ids`
    ///   length
    ///
    /// ## Arguments:
    /// * `sender_id`: the sender of `transfer_call`
    /// * `previous_owner_ids`: the accounts that owned the tokens prior to them being
    ///   transferred to this contract, which can differ from `sender_id` if using
    ///   Approval Management extension
    /// * `token_ids`: the `token_ids` argument given to `transfer_call`
    /// * `amounts`: the `amounts` argument given to `transfer_call`
    /// * `msg`: information necessary for this contract to know how to process the
    ///   request. This may include method names and/or arguments.
    ///
    /// Returns the number of unused tokens in integer form. For instance, if `amounts`
    /// is `[10]` but only 9 are needed, it will return `[1]`.
    fn mt_on_transfer(
        &mut self,
        sender_id: AccountId,
        previous_owner_ids: Vec<AccountId>,
        token_ids: Vec<TokenId>,
        amounts: Vec<U128>,
        msg: String,
    ) -> PromiseOrValue<Vec<U128>>;
}
