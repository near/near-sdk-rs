//! Smart contract with initialization function.

use borsh::{BorshDeserialize, BorshSerialize};
use near_sdk::{near_bindgen, NoDefault};

#[near_bindgen]
#[derive(NoDefault, BorshDeserialize, BorshSerialize)]
struct Incrementer {
    value: u32,
}

#[near_bindgen]
impl Incrementer {
    pub fn inc(&mut self, by: u32) {
        self.value += by;
    }
    #[init]
    pub fn new(starting_value: u32) -> Self {
        Self { value: starting_value }
    }
}

fn main() {}
