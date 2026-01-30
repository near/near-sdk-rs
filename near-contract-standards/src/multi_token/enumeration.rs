use crate::multi_token::token::Token;
use near_sdk::json_types::U128;
use near_sdk::{ext_contract, AccountId};

use super::metadata::MTBaseTokenMetadata;

/// Offers methods helpful in determining account ownership of multi tokens and provides a way
/// to page through tokens per owner, determine total supply, etc.
///
/// # Examples
///
/// ```
/// use near_sdk::{PanicOnDefault, AccountId, near};
/// use near_sdk::json_types::U128;
/// use near_contract_standards::multi_token::{MultiTokenEnumeration, TokenId, Token};
///
/// #[near(contract_state)]
/// #[derive(PanicOnDefault)]
/// pub struct Contract {
///    // tokens: MultiToken,
/// }
///
/// #[near]
/// impl MultiTokenEnumeration for Contract {
///     fn mt_tokens(&self, from_index: Option<U128>, limit: Option<u32>) -> Vec<Token> {
///         vec![] // Would need implementation
///     }
///
///     fn mt_tokens_for_owner(
///         &self,
///         account_id: AccountId,
///         from_index: Option<U128>,
///         limit: Option<u32>,
///     ) -> Vec<Token> {
///         vec![] // Would need implementation
///     }
/// }
/// ```
///
#[ext_contract(ext_mt_enumeration)]
pub trait MultiTokenEnumeration {
    /// Get a list of all tokens
    ///
    /// Arguments:
    /// * `from_index`: a string representing an unsigned 128-bit integer,
    ///    representing the starting index of tokens to return
    /// * `limit`: the maximum number of tokens to return
    ///
    /// Returns an array of `Token` objects, as described in the Core standard,
    /// and an empty array if there are no tokens
    fn mt_tokens(&self, from_index: Option<U128>, limit: Option<u32>) -> Vec<Token>;

    /// Get list of all tokens owned by a given account
    ///
    /// Arguments:
    /// * `account_id`: a valid NEAR account
    /// * `from_index`: a string representing an unsigned 128-bit integer,
    ///    representing the starting index of tokens to return
    /// * `limit`: the maximum number of tokens to return
    ///
    /// Returns a paginated list of all tokens owned by this account, and an empty array if there are no tokens
    fn mt_tokens_for_owner(
        &self,
        account_id: AccountId,
        from_index: Option<U128>,
        limit: Option<u32>,
    ) -> Vec<Token>;
}

/// Extension for enumeration that includes metadata if using the metadata extension.
#[ext_contract(ext_mt_enumeration_metadata)]
pub trait MultiTokenEnumerationMetadata {
    /// Get list of all base metadata for the contract
    ///
    /// Arguments:
    /// * `from_index`: a string representing an unsigned 128-bit integer,
    ///    representing the starting index of tokens to return
    /// * `limit`: the maximum number of tokens to return
    ///
    /// Returns an array of `MTBaseTokenMetadata` objects, as described in the Metadata standard,
    /// and an empty array if there are no tokens
    fn mt_tokens_base_metadata_all(
        &self,
        from_index: Option<U128>,
        limit: Option<u32>,
    ) -> Vec<MTBaseTokenMetadata>;
}
