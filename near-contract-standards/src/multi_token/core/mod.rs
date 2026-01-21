mod receiver;
pub mod resolver;

pub use self::receiver::{ext_mt_receiver, MultiTokenReceiver};
pub use self::resolver::{ext_mt_resolver, MultiTokenResolver};

use crate::multi_token::token::{Token, TokenId};
use near_sdk::ext_contract;
use near_sdk::json_types::U128;
use near_sdk::AccountId;
use near_sdk::PromiseOrValue;

/// Used for all multi tokens. The specification for the
/// [core multi token standard](https://nomicon.io/Standards/Tokens/MultiToken/Core) lays out the reasoning for each method.
/// It's important to check out [MultiTokenReceiver](crate::multi_token::core::MultiTokenReceiver)
/// and [MultiTokenResolver](crate::multi_token::core::MultiTokenResolver) to
/// understand how the cross-contract call work.
///
/// # Examples
///
/// ```
/// use near_sdk::{PanicOnDefault, AccountId, PromiseOrValue, near};
/// use near_sdk::json_types::U128;
/// use near_contract_standards::multi_token::{core::MultiTokenCore, TokenId, Token};
///
/// #[near(contract_state)]
/// #[derive(PanicOnDefault)]
/// pub struct Contract {
///    // tokens: MultiToken, // Would need implementation
/// }
/// ```
///
#[ext_contract(ext_mt_core)]
pub trait MultiTokenCore {
    /// Simple transfer. Transfer a given `token_id` from current owner to
    /// `receiver_id`.
    ///
    /// Requirements
    /// * Caller of the method must attach a deposit of 1 yoctoⓃ for security purposes
    /// * Caller must have greater than or equal to the `amount` being requested
    /// * Contract MUST panic if called by someone other than token owner or,
    ///   if using Approval Management, one of the approved accounts
    /// * `approval` is for use with Approval Management extension, see
    ///   that document for full explanation.
    /// * If using Approval Management, contract MUST nullify approved accounts on
    ///   successful transfer.
    ///
    /// Arguments:
    /// * `receiver_id`: the valid NEAR account receiving the token
    /// * `token_id`: the token to transfer
    /// * `amount`: the number of tokens to transfer
    /// * `approval`: optional tuple of (owner_id, approval_id) for approval management
    /// * `memo` (optional): for use cases that may benefit from indexing or
    ///    providing information for a transfer
    fn mt_transfer(
        &mut self,
        receiver_id: AccountId,
        token_id: TokenId,
        amount: U128,
        approval: Option<(AccountId, u64)>,
        memo: Option<String>,
    );

    /// Simple batch transfer. Transfer given `token_ids` from current owner to
    /// `receiver_id`.
    ///
    /// Requirements
    /// * Caller of the method must attach a deposit of 1 yoctoⓃ for security purposes
    /// * Caller must have greater than or equal to the `amounts` being requested for the given `token_ids`
    /// * Contract MUST panic if called by someone other than token owner or,
    ///   if using Approval Management, one of the approved accounts
    /// * `approvals` is for use with Approval Management extension, see
    ///   that document for full explanation.
    /// * If using Approval Management, contract MUST nullify approved accounts on
    ///   successful transfer.
    /// * Contract MUST panic if called with the length of `token_ids` not equal to `amounts`
    /// * Contract MUST panic if `approvals` is not `null` and does not equal the length of `token_ids`
    ///
    /// Arguments:
    /// * `receiver_id`: the valid NEAR account receiving the token
    /// * `token_ids`: the tokens to transfer
    /// * `amounts`: the number of tokens to transfer
    /// * `approvals`: optional array of (owner_id, approval_id) tuples for approval management
    /// * `memo` (optional): for use cases that may benefit from indexing or
    ///    providing information for a transfer
    fn mt_batch_transfer(
        &mut self,
        receiver_id: AccountId,
        token_ids: Vec<TokenId>,
        amounts: Vec<U128>,
        approvals: Option<Vec<Option<(AccountId, u64)>>>,
        memo: Option<String>,
    );

    /// Transfer token and call a method on a receiver contract. A successful
    /// workflow will end in a success execution outcome to the callback on the MT
    /// contract at the method `mt_resolve_transfer`.
    ///
    /// You can think of this as being similar to attaching native NEAR tokens to a
    /// function call. It allows you to attach any Multi Token in a call to a
    /// receiver contract.
    ///
    /// Requirements:
    /// * Caller of the method must attach a deposit of 1 yoctoⓃ for security
    ///   purposes
    /// * Caller must have greater than or equal to the `amount` being requested
    /// * Contract MUST panic if called by someone other than token owner or,
    ///   if using Approval Management, one of the approved accounts
    /// * The receiving contract must implement `mt_on_transfer` according to the
    ///   standard. If it does not, MT contract's `mt_resolve_transfer` MUST deal
    ///   with the resulting failed cross-contract call and roll back the transfer.
    /// * Contract MUST implement the behavior described in `mt_resolve_transfer`
    /// * `approval` is for use with Approval Management extension, see
    ///   that document for full explanation.
    /// * If using Approval Management, contract MUST nullify approved accounts on
    ///   successful transfer.
    ///
    /// Arguments:
    /// * `receiver_id`: the valid NEAR account receiving the token.
    /// * `token_id`: the token to send.
    /// * `amount`: the number of tokens to transfer
    /// * `approval`: optional tuple of (owner_id, approval_id) for approval management
    /// * `memo` (optional): for use cases that may benefit from indexing or
    ///    providing information for a transfer.
    /// * `msg`: specifies information needed by the receiving contract in
    ///    order to properly handle the transfer. Can indicate both a function to
    ///    call and the parameters to pass to that function.
    fn mt_transfer_call(
        &mut self,
        receiver_id: AccountId,
        token_id: TokenId,
        amount: U128,
        approval: Option<(AccountId, u64)>,
        memo: Option<String>,
        msg: String,
    ) -> PromiseOrValue<Vec<U128>>;

    /// Transfer tokens and call a method on a receiver contract. A successful
    /// workflow will end in a success execution outcome to the callback on the MT
    /// contract at the method `mt_resolve_transfer`.
    ///
    /// Requirements:
    /// * Caller of the method must attach a deposit of 1 yoctoⓃ for security
    ///   purposes
    /// * Caller must have greater than or equal to the `amounts` being requested
    /// * Contract MUST panic if called by someone other than token owner or,
    ///   if using Approval Management, one of the approved accounts
    /// * The receiving contract must implement `mt_on_transfer` according to the
    ///   standard. If it does not, MT contract's `mt_resolve_transfer` MUST deal
    ///   with the resulting failed cross-contract call and roll back the transfer.
    /// * Contract MUST implement the behavior described in `mt_resolve_transfer`
    /// * `approvals` is for use with Approval Management extension, see
    ///   that document for full explanation.
    /// * If using Approval Management, contract MUST nullify approved accounts on
    ///   successful transfer.
    /// * Contract MUST panic if called with the length of `token_ids` not equal to `amounts`
    /// * Contract MUST panic if `approvals` is not `null` and does not equal the length of `token_ids`
    ///
    /// Arguments:
    /// * `receiver_id`: the valid NEAR account receiving the token.
    /// * `token_ids`: the tokens to send
    /// * `amounts`: the number of tokens to transfer
    /// * `approvals`: optional array of (owner_id, approval_id) tuples for approval management
    /// * `memo` (optional): for use cases that may benefit from indexing or
    ///    providing information for a transfer.
    /// * `msg`: specifies information needed by the receiving contract in
    ///    order to properly handle the transfer.
    fn mt_batch_transfer_call(
        &mut self,
        receiver_id: AccountId,
        token_ids: Vec<TokenId>,
        amounts: Vec<U128>,
        approvals: Option<Vec<Option<(AccountId, u64)>>>,
        memo: Option<String>,
        msg: String,
    ) -> PromiseOrValue<Vec<U128>>;

    /// Returns the tokens with the given `token_ids` or `null` if no such token.
    fn mt_token(&self, token_ids: Vec<TokenId>) -> Vec<Option<Token>>;

    /// Returns the balance of an account for the given `token_id`.
    /// The balance though wrapped in quotes and treated like a string,
    /// the number will be stored as an unsigned integer with 128 bits.
    ///
    /// Arguments:
    /// * `account_id`: the NEAR account that owns the token.
    /// * `token_id`: the token to retrieve the balance from
    fn mt_balance_of(&self, account_id: AccountId, token_id: TokenId) -> U128;

    /// Returns the balances of an account for the given `token_ids`.
    /// The balances though wrapped in quotes and treated like strings,
    /// the numbers will be stored as unsigned integers with 128 bits.
    ///
    /// Arguments:
    /// * `account_id`: the NEAR account that owns the tokens.
    /// * `token_ids`: the tokens to retrieve the balance from
    fn mt_batch_balance_of(&self, account_id: AccountId, token_ids: Vec<TokenId>) -> Vec<U128>;

    /// Returns the token supply with the given `token_id` or `null` if no such token exists.
    /// The supply though wrapped in quotes and treated like a string, the number will be stored
    /// as an unsigned integer with 128 bits.
    fn mt_supply(&self, token_id: TokenId) -> Option<U128>;

    /// Returns the token supplies with the given `token_ids`, a string value is returned or `null`
    /// if no such token exists. The supplies though wrapped in quotes and treated like strings,
    /// the numbers will be stored as unsigned integers with 128 bits.
    fn mt_batch_supply(&self, token_ids: Vec<TokenId>) -> Vec<Option<U128>>;
}
