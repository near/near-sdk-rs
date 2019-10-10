//! Method with non-deserializable argument type.

use near_bindgen::near_bindgen;
use borsh::{BorshDeserialize, BorshSerialize};

#[near_bindgen]
#[derive(Default, BorshDeserialize, BorshSerialize)]
struct Storage {
    data: Vec<u64>,
}

#[near_bindgen]
impl Storage {
    pub fn insert(&mut self) -> impl Iterator<Item=&u64> {
        self.data.iter()
    }
}

fn main() {}
