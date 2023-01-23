//! Method signature uses lifetime.
use std::borrow::Cow;

use borsh::{BorshDeserialize, BorshSerialize};
use near_sdk::near_bindgen;

#[near_bindgen]
#[derive(Default, BorshDeserialize, BorshSerialize)]
struct Ident {
    value: u32,
}

#[near_bindgen]
impl Ident {
    pub fn is_ident<'a>(&self, other: &'a u32) -> Option<&'a u32> {
        if *other == self.value {
            Some(other)
        } else {
            None
        }
    }
    pub fn nested_lifetime<'a, 'b, 'c>(&self) -> Option<&'a Cow<'b, [&'c u8; 32]>> {
        None
    }
}

fn main() {}
