use near_sdk::json_types::U128;
use near_sdk::AccountId;

pub mod enumeration_impl;

use super::{metadata::MtContractMetadata, token::Token};

/// Enumeration extension for NEP-245
/// See specs here -> <https://github.com/shipsgold/NEPs/blob/master/specs/Standards/MultiToken/Enumeration.md>
pub trait MultiTokenEnumeration {
    /// Get a list of all tokens (with pagination)
    ///
    /// # Arguments:
    /// * `from_index` - Index to start from, defaults to 0 if not provided
    /// * `limit` - The maximum number of tokens to return
    ///
    /// returns: List of [Token]s.
    ///
    fn mt_tokens(&self, from_index: Option<U128>, limit: Option<u64>) -> Vec<Token>;

    /// Get list of all tokens by a given account
    ///
    /// # Arguments:
    /// * `account_id`: a valid NEAR account
    /// * `from_index` - Index to start from, defaults to 0 if not provided
    /// * `limit` - The maximum number of tokens to return
    ///
    /// returns: List of [Token]s owner by user
    ///
    fn mt_tokens_for_owner(
        &self,
        account_id: AccountId,
        from_index: Option<U128>,
        limit: Option<u64>,
    ) -> Vec<Token>;
}

/// The contract must implement the following view methods if using metadata extension
pub trait MultiTokenEnumerationMetadata {
    /// Get list of all base metadata for the contract
    ///
    /// Arguments:
    /// * `from_index`: a string representing an unsigned 128-bit integer,
    ///    representing the starting index of tokens to return
    /// * `limit`: the maximum number of tokens to return
    ///
    /// Returns an array of `MTBaseTokenMetadata` objects, as described in the Metadata standard, and an empty array if there are no tokens
    fn mt_tokens_base_metadata_all(
        &self,
        from_index: Option<U128>,
        limit: Option<u64>,
    ) -> Vec<MtContractMetadata>;
}
