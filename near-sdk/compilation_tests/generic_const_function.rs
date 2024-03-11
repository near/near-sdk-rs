//! Functions can't use const generics.

use near_sdk::near;


#[derive(Default)]
#[near(contract_state)]
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
