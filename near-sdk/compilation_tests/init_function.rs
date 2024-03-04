//! Smart contract with initialization function.

use near_sdk::near_bindgen;
use borsh::{BorshDeserialize, BorshSerialize};

#[near(contract_state)]
#[derive(BorshDeserialize, BorshSerialize)]
struct Incrementer {
    value: u32,
}

#[near]
impl Incrementer {
    pub fn inc(&mut self, by: u32) {
        self.value += by;
    }
    #[init]
    pub fn new(starting_value: u32) -> Self {
        Self {
            value: starting_value
        }
    }
}

fn main() {}
