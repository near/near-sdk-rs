//! Method signature uses Self.

use near_sdk::near;


#[derive(Default)]
#[near(contract_state, serializers=[json])]
pub struct Ident {
    value: u32,
}

#[near]
impl Ident {
    pub fn plain_arg(_value: Option<Self>, _value2: Self) {
        unimplemented!()
    }
}

fn main() {}
