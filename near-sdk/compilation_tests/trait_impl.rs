//! Smart contract that implements trait.

use near_sdk::near_sdk;
use borsh::{BorshDeserialize, BorshSerialize};

#[near_sdk]
#[derive(Default, BorshDeserialize, BorshSerialize)]
struct Incrementer {
    value: u32,
}

pub trait Zeroable {
    fn set_to_zero(&mut self);
}

#[near_sdk]
impl Incrementer {
    pub fn inc(&mut self, by: u32) {
        self.value += by;
    }
}

#[near_sdk]
impl Zeroable for Incrementer {
    fn set_to_zero(&mut self) {
        self.value = 0;
    }
}

fn main() {}
