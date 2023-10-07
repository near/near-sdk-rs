mod payout_impl;

use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::json_types::U128;
use near_sdk::AccountId;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

type BasisPoint = u16;

/// This struct represents NFT royalty payout for each receiver.
#[derive(Default, BorshDeserialize, BorshSerialize, Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct Payout {
    pub payout: HashMap<AccountId, U128>,
}

/// This struct represents percentage of total royalty per receiver as well as the total percentage
/// of distributed royalties based on incoming payment.
#[derive(Deserialize, Serialize, BorshDeserialize, BorshSerialize, Default, Debug)]
#[serde(crate = "near_sdk::serde")]
pub struct Royalties {
    key_prefix: Vec<u8>,
    /// A mapping of accounts to the percentage of total royalty to be distributed.
    pub accounts: HashMap<AccountId, BasisPoint>,
    /// Total percent of incoming balance used for royalties.
    pub percent: BasisPoint,
}

/// An interface allowing non-fungible token contracts to request that financial contracts pay-out
/// multiple receivers, enabling flexible royalty implementations.
///
/// [Royalties and Payouts standard]: <https://nomicon.io/Standards/Tokens/NonFungibleToken/Payout>
///
/// # Examples
///
/// ```
/// use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
/// use near_sdk::{PanicOnDefault, AccountId, PromiseOrValue, near_bindgen, assert_one_yocto};
/// use near_contract_standards::non_fungible_token::{core::NonFungibleTokenCore, NonFungibleToken, NonFungibleTokenPayout, payout::Payout, TokenId, Token};
/// use near_sdk::json_types::U128;
///
/// #[near_bindgen]
/// #[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
/// pub struct Contract {
///    tokens: NonFungibleToken,
///}
/// #[near_bindgen]
/// impl NonFungibleTokenCore for Contract {
///     #[payable]
///    fn nft_transfer(&mut self, receiver_id: AccountId, token_id: TokenId, approval_id: Option<u64>, memo: Option<String>) {
///        self.tokens.nft_transfer(receiver_id, token_id, approval_id, memo);
///    }
///
///    #[payable]
///    fn nft_transfer_call(&mut self, receiver_id: AccountId, token_id: TokenId, approval_id: Option<u64>, memo: Option<String>, msg: String) -> PromiseOrValue<bool> {
///        self.tokens.nft_transfer_call(receiver_id, token_id, approval_id, memo, msg)
///    }
///
///    fn nft_token(&self, token_id: TokenId) -> Option<Token> {
///        self.tokens.nft_token(token_id)
///    }
///}
/// #[near_bindgen]
/// impl NonFungibleTokenPayout for Contract {
///     #[allow(unused_variables)]
///     fn nft_payout(
///         &self,
///         token_id: String,
///         balance: U128,
///         max_len_payout: Option<u32>,
///     ) -> Payout {
///         self.tokens.nft_payout(token_id, balance, max_len_payout)
///     }
///     #[payable]
///     fn nft_transfer_payout(
///         &mut self,
///         receiver_id: AccountId,
///         token_id: String,
///         approval_id: Option<u64>,
///         memo: Option<String>,
///         balance: U128,
///         max_len_payout: Option<u32>,
///     ) -> Payout {
///         self.tokens.nft_transfer_payout(
///             receiver_id,
///             token_id,
///             approval_id,
///             memo,
///             balance,
///             max_len_payout,
///         )
///     }
/// }
/// ```
///
pub trait NonFungibleTokenPayout {
    fn nft_payout(&self, token_id: String, balance: U128, max_len_payout: Option<u32>) -> Payout;
    /// Given a `token_id` and NEAR-denominated balance, transfer the token
    /// and return the `Payout` struct for the given token. Panic if the
    /// length of the payout exceeds `max_len_payout.`
    fn nft_transfer_payout(
        &mut self,
        receiver_id: AccountId,
        token_id: String,
        approval_id: Option<u64>,
        memo: Option<String>,
        balance: U128,
        max_len_payout: Option<u32>,
    ) -> Payout;
}
