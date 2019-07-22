//! Regular smart contract.

#![feature(const_vec_new)]
use near_bindgen::near_bindgen;
use serde::{Deserialize, Serialize};

#[near_bindgen]
#[derive(Default, Serialize, Deserialize)]
struct Incrementer {
    value: u32,
}

#[near_bindgen]
impl Incrementer {
    pub fn inc(&mut self, by: u32) {
        self.value += by;
    }
}

fn main() {}
