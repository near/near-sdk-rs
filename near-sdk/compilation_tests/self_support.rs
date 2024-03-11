//! Method signature uses Self.

use near_sdk::near;

#[derive(Default)]
#[near(contract_state, serializers=[json])]
pub struct Ident {
    value: u32,
}

#[near]
impl Ident {
    #[init]
    pub fn plain_arg(_a: Self) -> Self {
        unimplemented!()
    }

    #[init]
    pub fn plain_ret() -> Self {
        unimplemented!()
    }

    #[init]
    pub fn plain_arg_ret(a: Self) -> Self {
        a
    }

    #[init]
    pub fn nested_arg(_a: Vec<Self>) -> Self {
        unimplemented!()
    }

    #[init]
    pub fn nested_ret() -> Vec<Self> {
        unimplemented!()
    }

    #[init]
    pub fn nested_arg_ret(a: Vec<Self>) -> Vec<Self> {
        a
    }

    #[init]
    pub fn deeply_nested_arg(_a: Option<[(Self, Result<Self, ()>); 2]>) -> Self {
        unimplemented!()
    }

    #[init]
    pub fn deeply_nested_ret() -> Option<[(Self, Result<Self, ()>); 2]> {
        unimplemented!()
    }

    #[init]
    pub fn deeply_nested_arg_ret(
        a: Option<[(Self, Result<Self, ()>); 2]>,
    ) -> Option<[(Self, Result<Self, ()>); 2]> {
        a
    }
}

fn main() {}
