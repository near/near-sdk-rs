//! Functions can't use generics.

use near_sdk::near;


#[derive(Default)]
#[near(contract_state)]
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
