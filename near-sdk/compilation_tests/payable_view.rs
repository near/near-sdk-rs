//! Payable view are not valid

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
