//! Payable view are not valid

use borsh::{BorshDeserialize, BorshSerialize};

#[near(contract_state)]
#[derive(Default, BorshDeserialize, BorshSerialize)]
struct Test {}

#[near]
impl Test {
    #[payable]
    pub fn pay(&self) {}
}

fn main() {}
