use crate::multi_token::token::TokenId;
use near_sdk::json_types::U128;
use near_sdk::{ext_contract, AccountId, PromiseOrValue};

/// Used when a multi token is transferred using `mt_transfer_call` or `mt_batch_transfer_call`.
/// This trait is implemented on the receiving contract, not on the MT contract.
#[ext_contract(ext_mt_receiver)]
pub trait MultiTokenReceiver {
    /// Take some action after receiving a multi token
    ///
    /// Requirements:
    /// * Contract MUST restrict calls to this function to a set of whitelisted
    ///   contracts
    /// * Contract MUST panic if `token_ids` length does not equal `amounts` length
    /// * Contract MUST panic if `previous_owner_ids` length does not equal `token_ids` length
    ///
    /// Arguments:
    /// * `sender_id`: the sender of `mt_transfer_call`
    /// * `previous_owner_ids`: the accounts that owned the tokens prior to it being
    ///   transferred to this contract, which can differ from `sender_id` if using
    ///   Approval Management extension
    /// * `token_ids`: the `token_ids` argument given to `mt_transfer_call`
    /// * `amounts`: the `amounts` argument given to `mt_transfer_call`
    /// * `msg`: information necessary for this contract to know how to process the
    ///   request. This may include method names and/or arguments.
    ///
    /// Returns the number of unused tokens in string form. For instance, if `amounts`
    /// is `["10"]` but only 9 are needed, it will return `["1"]`.
    fn mt_on_transfer(
        &mut self,
        sender_id: AccountId,
        previous_owner_ids: Vec<AccountId>,
        token_ids: Vec<TokenId>,
        amounts: Vec<U128>,
        msg: String,
    ) -> PromiseOrValue<Vec<U128>>;
}
