//! Regular smart contract.

use near_sdk::near;

#[derive(Default)]
#[near(contract_state)]
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
