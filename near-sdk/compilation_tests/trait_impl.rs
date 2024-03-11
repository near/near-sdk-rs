//! Smart contract that implements trait.

use borsh::{BorshDeserialize, BorshSerialize};

#[near(contract_state)]
#[derive(Default, BorshDeserialize, BorshSerialize)]
struct Incrementer {
    value: u32,
}

pub trait Zeroable {
    fn set_to_zero(&mut self);
}

#[near]
impl Incrementer {
    pub fn inc(&mut self, by: u32) {
        self.value += by;
    }
}

#[near]
impl Zeroable for Incrementer {
    fn set_to_zero(&mut self) {
        self.value = 0;
    }
}

fn main() {}
