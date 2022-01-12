//! Method with non-deserializable argument type.

use borsh::{BorshDeserialize, BorshSerialize};
use near_sdk::{near_bindgen, PanicOnDefault};

#[near_bindgen]
#[derive(BorshSerialize, BorshDeserialize, PanicOnDefault)]
struct Storage {}

#[near_bindgen]
impl Storage {
    pub fn insert(&mut self, (a, b): (u8, u32)) {}
}

fn main() {}
