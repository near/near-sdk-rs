//! A smart contract setting configuration parameter `blacklist_ext_fn_attrs`.

use borsh::{BorshDeserialize, BorshSerialize};
use near_sdk::near_bindgen;

#[near_bindgen]
#[derive(Default, BorshDeserialize, BorshSerialize)]
struct Incrementer {
    value: u32,
}

#[near_bindgen(blacklist_ext_fn_attrs(inline, forbid))]
impl Incrementer {
    #[inline] // a bare attribute
    #[allow(missing_docs)] // an attribute with additional input
    pub fn inc(&mut self, by: u32) {
        self.value += by;
    }
}

fn main() {}
