//! Regular smart contract.

use borsh::{BorshDeserialize, BorshSerialize};
use near_sdk::near_bindgen;

#[near_bindgen]
#[derive(Default, BorshDeserialize, BorshSerialize)]
struct Incrementer {
    value: u32,
}

#[near_bindgen]
impl Incrementer {
    #[private]
    pub fn inc(&mut self, by: u32) {
        self.value += by;
    }
}

fn main() {}
