#![allow(unused_imports)]

use borsh::{BorshDeserialize, BorshSerialize};
use near_sdk::{metadata, near_bindgen};
metadata! {
FOOBAR

#[near_bindgen]
#[derive(Default, BorshDeserialize, BorshSerialize)]
struct Incrementer {
    value: u32,
}

#[near_bindgen]
impl Incrementer {
    pub fn inc(&mut self, by: u32) {
        self.value += by;
    }
}
}

fn main() {}
