//! Smart contract with initialization function that uses wrong syntax.

#![feature(const_vec_new)]
use near_bindgen::near_bindgen;
use borsh::{BorshDeserialize, BorshSerialize};

#[near_bindgen]
#[derive(Default, BorshDeserialize, BorshSerialize)]
struct Incrementer {
    value: u32,
}

#[near_bindgen(initialize => new)]
impl Incrementer {
    pub fn inc(&mut self, by: u32) {
        self.value += by;
    }
    pub fn new(starting_value: u32) -> Self {
        Self {
            value: starting_value
        }
    }
}

fn main() {}
