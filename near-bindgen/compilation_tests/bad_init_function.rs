//! Smart contract with initialization function that has bad signature.

#![feature(const_vec_new)]
use near_bindgen::near_bindgen;
use borsh::{BorshDeserialize, BorshSerialize};

#[near_bindgen]
#[derive(Default, BorshDeserialize, BorshSerialize)]
struct Incrementer {
    value: u32,
}

#[near_bindgen(init => new)]
impl Incrementer {
    pub fn inc(&mut self, by: u32) {
        self.value += by;
    }
    pub fn new(&mut self, starting_value: u32) -> Self {
        Self {
            value: starting_value
        }
    }
}

fn main() {}
