//! Functions can't use generics.

use borsh::{BorshDeserialize, BorshSerialize};

#[near(contract_state)]
#[derive(Default, BorshDeserialize, BorshSerialize)]
struct Ident {
    value: u32,
}

#[near]
impl Ident {
    pub fn is_ident<T>(&self, val: T) -> T {
        val
    }
}

fn main() {}
