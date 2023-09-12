//! Even though it might feel unintuitive, a method can be both private and init.
//! See: https://github.com/near/near-sdk-rs/issues/1040#issuecomment-1687126452

use borsh::{BorshDeserialize, BorshSerialize};
use near_sdk::near_bindgen;

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize)]
struct Incrementer {
    value: u32,
}

#[near_bindgen]
impl Incrementer {
    #[private]
    #[init]
    pub fn new(starting_value: u32) -> Self {
        Self { value: starting_value }
    }
}

fn main() {}
