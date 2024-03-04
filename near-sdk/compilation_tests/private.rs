//! Regular smart contract.

use borsh::{BorshDeserialize, BorshSerialize};
use near_sdk::near_bindgen;

#[near(contract_state)]
#[derive(Default, BorshDeserialize, BorshSerialize)]
struct Incrementer {
    value: u32,
}

#[near]
impl Incrementer {
    #[private]
    pub fn inc(&mut self, by: u32) {
        self.value += by;
    }
}

fn main() {}
