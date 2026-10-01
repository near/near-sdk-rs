//! Payable view methods are now valid since nearcore PR #8433

use near_sdk::near;


#[derive(Default)]
#[near(contract_state)]
struct Test {}

#[near]
impl Test {
    #[payable]
    pub fn pay(&self) {}
}

fn main() {}
