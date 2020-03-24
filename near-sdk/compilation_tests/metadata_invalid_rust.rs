use near_sdk::{near_bindgen, metadata};
use borsh::{BorshDeserialize, BorshSerialize};
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
