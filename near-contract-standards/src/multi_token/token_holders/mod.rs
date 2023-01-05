use crate::multi_token::token::TokenId;
use near_sdk::{json_types::U128, AccountId};

mod token_holders_impl;

pub use token_holders_impl::*;

pub trait MultiTokenHolders {
    /// Get a list of all token holders (with pagination)
    ///
    /// # Arguments:
    /// * `token_id` - ID of the token
    /// * `from_index`: a string representing an unsigned 128-bit integer,
    ///    representing the starting index of accounts to return
    /// * `limit`: the maximum number of accounts to return
    /// returns: List of [AccountId]s.
    ///
    fn mt_token_holders(
        &self,
        token_id: TokenId,
        from_index: Option<U128>,
        limit: Option<u64>,
    ) -> Vec<AccountId>;
}
