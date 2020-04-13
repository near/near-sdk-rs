//! Payable view are not valid

use borsh::{BorshDeserialize, BorshSerialize};
use near_sdk::near_bindgen;

#[near_bindgen]
#[derive(Default, BorshDeserialize, BorshSerialize)]
struct Test {}

#[near_bindgen]
impl Test {
    #[payable]
    pub fn pay(&self) {}
}

fn main() {}
