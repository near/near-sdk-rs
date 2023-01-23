//! Functions can't use const generics.

use borsh::{BorshDeserialize, BorshSerialize};
use near_sdk::near_bindgen;

#[near_bindgen]
#[derive(Default, BorshDeserialize, BorshSerialize)]
struct Ident {
    value: u32,
}

#[near_bindgen]
impl Ident {
    pub fn is_ident_const<const N: usize>(&self, val: [u32; N]) -> [u32; N] {
        val
    }
}

fn main() {}
