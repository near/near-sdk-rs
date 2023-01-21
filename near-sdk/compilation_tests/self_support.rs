//! Method signature uses Self.

use borsh::{BorshDeserialize, BorshSerialize};
use near_sdk::near_bindgen;
use serde::{Deserialize, Serialize};

#[near_bindgen]
#[derive(Default, Serialize, Deserialize)]
pub struct Ident {
    value: u32,
}

#[near_bindgen]
impl Ident {
    pub fn plain_arg(a: Self) {}
    pub fn plain_ret() -> Self {
        todo!()
    }
    pub fn plain_arg_ret(a: Self) -> Self {
        todo!()
    }
    pub fn nested_arg(a: Vec<Self>) {}
    pub fn nested_ret() -> Vec<Self> {
        todo!()
    }
    pub fn nested_arg_ret(a: Vec<Self>) -> Vec<Self> {
        todo!()
    }
    pub fn deeply_nested_arg(a: Option<[(Self, Result<Self, ()>); 2]>) {}
    pub fn deeply_nested_ret() -> Option<[(Self, Result<Self, ()>); 2]> {
        todo!()
    }
    pub fn deeply_nested_arg_ret(
        a: Option<[(Self, Result<Self, ()>); 2]>,
    ) -> Option<[(Self, Result<Self, ()>); 2]> {
        todo!()
    }
}

fn main() {}
