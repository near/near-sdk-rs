//! Rust contract that uses conditional compilation.

use near_sdk::near_sdk;
use borsh::{BorshDeserialize, BorshSerialize};

#[near_sdk]
#[derive(Default, BorshDeserialize, BorshSerialize)]
struct Incrementer {
    value: u32,
}

#[near_sdk(init => new)]
impl Incrementer {
    #[cfg(feature = "myfeature")]
    pub fn new() -> Self {
        Self {value: 0}
    }

    #[cfg(not(feature = "myfeature"))]
    pub fn new() -> Self {
        Self {value: 1}
    }

    #[cfg(feature = "myfeature")]
    pub fn inc(&mut self, by: u32) {
        self.value += by;
    }

    #[cfg(not(feature = "myfeature"))]
    pub fn inc(&mut self, by: u32) {
        self.value += by;
    }
}

fn main() {}
