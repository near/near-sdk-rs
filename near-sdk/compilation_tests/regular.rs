//! Regular smart contract.

use near_sdk::near;

#[near(contract_state)]
#[derive(Default)]
struct Incrementer {
    value: u32,
}

#[near]
impl Incrementer {
    pub fn inc(&mut self, by: u32) {
        self.value += by;
    }
}

fn main() {}
