//! Regular smart contract.

use near_sdk::near;

#[derive(Default)]
#[near(contract_state)]
struct Incrementer {
    value: u32,
}

#[near]
impl Incrementer {
    #[init]
    #[deny_unknown_arguments]
    pub fn new(starting_value: u32) -> Self {
        Self { value: starting_value }
    }

    #[deny_unknown_arguments]
    pub fn inc(&mut self, by: u32) {
        self.value += by;
    }

    #[deny_unknown_arguments]
    pub fn inc_view(&self, by: u32) -> u32 {
        self.value + by
    }
}

fn main() {}
