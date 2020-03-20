use near_sdk::{near_sdk, metadata};
use borsh::{BorshDeserialize, BorshSerialize};
metadata! {
FOOBAR

#[near_sdk]
#[derive(Default, BorshDeserialize, BorshSerialize)]
struct Incrementer {
    value: u32,
}

#[near_sdk]
impl Incrementer {
    pub fn inc(&mut self, by: u32) {
        self.value += by;
    }
}
}

fn main() {}
