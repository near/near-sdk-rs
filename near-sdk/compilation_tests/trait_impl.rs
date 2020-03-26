//! Smart contract that implements trait.

use near_sdk::near_bindgen;
use borsh::{BorshDeserialize, BorshSerialize};

#[near_bindgen]
#[derive(Default, BorshDeserialize, BorshSerialize)]
struct Incrementer {
    value: u32,
}

pub trait Zeroable {
    fn set_to_zero(&mut self);
}

#[near_bindgen]
impl Incrementer {
    pub fn inc(&mut self, by: u32) {
        self.value += by;
    }
}

#[near_bindgen]
impl Zeroable for Incrementer {
    fn set_to_zero(&mut self) {
        self.value = 0;
    }
}

fn main() {}
