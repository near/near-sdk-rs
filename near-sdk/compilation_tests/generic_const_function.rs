//! Functions can't use const generics.

use borsh::{BorshDeserialize, BorshSerialize};

#[near(contract_state)]
#[derive(Default, BorshDeserialize, BorshSerialize)]
struct Ident {
    value: u32,
}

#[near]
impl Ident {
    pub fn is_ident_const<const N: usize>(&self, val: [u32; N]) -> [u32; N] {
        val
    }
}

fn main() {}
