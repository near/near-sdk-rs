//! Method with non-deserializable argument type.

#![feature(const_vec_new)]
use near_bindgen::near_bindgen;
use serde::{Deserialize, Serialize};

#[near_bindgen]
#[derive(Default, Serialize, Deserialize)]
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
