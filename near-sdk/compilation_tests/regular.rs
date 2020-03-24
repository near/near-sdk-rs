//! Regular smart contract.

use near_sdk::near_bindgen;
use borsh::{BorshDeserialize, BorshSerialize};

#[near_bindgen]
#[derive(Default, BorshDeserialize, BorshSerialize)]
struct Incrementer {
    value: u32,
}

#[near_bindgen]
impl Incrementer {
    pub fn inc(&mut self, by: u32) {
        self.value += by;
    }
}

fn main() {}
