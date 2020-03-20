//! Method signature uses lifetime.

use near_sdk::near_sdk;
use borsh::{BorshDeserialize, BorshSerialize};

#[near_sdk]
#[derive(Default, BorshDeserialize, BorshSerialize)]
struct Ident {
    value: u32,
}

#[near_sdk]
impl Ident {
    pub fn is_ident<'a>(&self, other: &'a u32) -> Option<&'a u32> {
        if *other == self.value {
            Some(other)
        } else {
            None
        }
    }
}

fn main() {}
