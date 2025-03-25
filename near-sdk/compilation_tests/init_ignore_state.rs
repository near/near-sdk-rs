//! Smart contract with initialization function.

use near_sdk::near;

#[near(contract_state)]
struct Incrementer {
    value: u32,
}

#[near]
impl Incrementer {
    pub fn inc(&mut self, by: u32) {
        self.value += by;
    }
    #[init(ignore_state)]
    pub fn new(starting_value: u32) -> Self {
        Self { value: starting_value }
    }
}

fn main() {}
