//! Regular smart contract.

use near_sdk::near_sdk;
use borsh::{BorshDeserialize, BorshSerialize};

#[near_sdk]
#[derive(Default, BorshDeserialize, BorshSerialize)]
struct Incrementer {
    value: u32,
}

#[near_sdk]
impl Incrementer {
    pub fn inc(&mut self, by: u32) {
        self.value += by;
    }
}

fn main() {}
