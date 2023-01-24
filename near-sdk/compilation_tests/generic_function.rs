//! Functions can't use generics.

use borsh::{BorshDeserialize, BorshSerialize};
use near_sdk::near_bindgen;

#[near_bindgen]
#[derive(Default, BorshDeserialize, BorshSerialize)]
struct Ident {
    value: u32,
}

#[near_bindgen]
impl Ident {
    pub fn is_ident<T>(&self, val: T) -> T {
        val
    }
}

fn main() {}
