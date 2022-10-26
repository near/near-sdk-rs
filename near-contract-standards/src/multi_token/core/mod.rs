/*! Multi-Token Implementation (ERC-1155)

*/

mod core_impl;
mod receiver;
mod resolver;

pub use self::core_impl::*;
pub use self::receiver::*;
pub use self::resolver::*;

use crate::multi_token::token::TokenId;
use near_sdk::json_types::U128;
use near_sdk::{AccountId, PromiseOrValue};

use super::token::Token;

/// Describes functionality according to this - https://eips.ethereum.org/EIPS/eip-1155
/// And this - <https://github.com/shipsgold/NEPs/blob/master/specs/Standards/MultiToken/Core.md>
pub trait MultiTokenCore {
    /// Make a single transfer
    ///
    /// # Arguments
    ///
    /// * `receiver_id`: the valid NEAR account receiving the token
    /// * `token_id`: ID of the token to transfer
    /// * `amount`: the number of tokens to transfer
    /// * `approval`: owner account and ID of approval for signer
    /// * `memo`: Used as context
    /// returns: ()
    ///
    fn mt_transfer(
        &mut self,
        receiver_id: AccountId,
        token_id: TokenId,
        amount: U128,
        approval: Option<(AccountId, u64)>,
        memo: Option<String>,
    );

    // Make a batch transfer
    fn mt_batch_transfer(
        &mut self,
        receiver_id: AccountId,
        token_ids: Vec<TokenId>,
        amounts: Vec<U128>,
        approvals: Option<Vec<Option<(AccountId, u64)>>>,
        memo: Option<String>,
    );

    /// Transfer MT and call a method on receiver contract. A successful
    /// workflow will end in a success execution outcome to the callback on the MT
    /// contract at the method `resolve_transfer`.
    ///
    /// # Arguments
    ///
    /// * `receiver_id`: NEAR account receiving MT
    /// * `token_id`: Token to send
    /// * `amount`: How much to send
    /// * `approval`: owner account and ID of approval for signer
    /// * `memo`: Used as context
    /// * `msg`: Additional msg that will be passed to receiving contract
    ///
    /// returns: PromiseOrValue<U128>
    ///
    fn mt_transfer_call(
        &mut self,
        receiver_id: AccountId,
        token_id: TokenId,
        amount: U128,
        approval: Option<(AccountId, u64)>,
        memo: Option<String>,
        msg: String,
    ) -> PromiseOrValue<U128>;

    // Batched version of mt_transfer_call
    fn mt_batch_transfer_call(
        &mut self,
        receiver_id: AccountId,
        token_ids: Vec<TokenId>,
        amounts: Vec<U128>,
        approvals: Option<Vec<Option<(AccountId, u64)>>>,
        memo: Option<String>,
        msg: String,
    ) -> PromiseOrValue<Vec<U128>>;

    // View Methods
    fn mt_token(&self, token_ids: Vec<TokenId>) -> Vec<Option<Token>>;

    fn mt_balance_of(&self, account_id: AccountId, token_id: TokenId) -> U128;

    fn mt_batch_balance_of(&self, account_id: AccountId, token_ids: Vec<TokenId>) -> Vec<U128>;

    fn mt_supply(&self, token_id: TokenId) -> Option<U128>;

    fn mt_batch_supply(&self, token_ids: Vec<TokenId>) -> Vec<Option<U128>>;
}
