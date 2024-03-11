//! Regular smart contract.

use borsh::{BorshDeserialize, BorshSerialize};

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
