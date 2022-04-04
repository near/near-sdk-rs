mod payout_impl;

use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::json_types::U128;
use near_sdk::AccountId;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
#[derive(Default, BorshDeserialize, BorshSerialize, Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]

pub struct Payout {
    pub payout: HashMap<AccountId, U128>,
}
#[derive(Deserialize, Serialize, BorshDeserialize, BorshSerialize, Default, Debug)]
#[serde(crate = "near_sdk::serde")]
pub struct Royalties {
    key_prefix: Vec<u8>,
    pub accounts: HashMap<AccountId, u8>,
    pub percent: u8,
}
/// Offers methods helpful in determining account ownership of NFTs and provides a way to page through NFTs per owner, determine total supply, etc.
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
