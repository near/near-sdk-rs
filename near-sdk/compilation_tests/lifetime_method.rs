//! Method signature uses lifetime.

use near_sdk::near;

#[near(contract_state)]
#[derive(Default)]
struct Ident {
    value: u32,
}

#[near]
impl Ident {
    pub fn is_ident<'a>(&self, other: &'a u32) -> Option<u32> {
        if *other == self.value {
            Some(*other)
        } else {
            None
        }
    }
}

fn main() {}
