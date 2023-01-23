//! Method signature uses Self.

use near_sdk::near_bindgen;
use serde::{Deserialize, Serialize};

#[near_bindgen]
#[derive(Default, Serialize, Deserialize)]
pub struct Ident {
    value: u32,
}

#[near_bindgen]
impl Ident {
    pub fn plain_arg(_a: Self) {
        unimplemented!()
    }
    pub fn plain_ret() -> Self {
        unimplemented!()
    }
    pub fn plain_arg_ret(a: Self) -> Self {
        a
    }
    pub fn nested_arg(_a: Vec<Self>) {
        unimplemented!()
    }
    pub fn nested_ret() -> Vec<Self> {
        unimplemented!()
    }
    pub fn nested_arg_ret(a: Vec<Self>) -> Vec<Self> {
        a
    }
    pub fn deeply_nested_arg(_a: Option<[(Self, Result<Self, ()>); 2]>) {
        unimplemented!()
    }
    pub fn deeply_nested_ret() -> Option<[(Self, Result<Self, ()>); 2]> {
        unimplemented!()
    }
    pub fn deeply_nested_arg_ret(
        a: Option<[(Self, Result<Self, ()>); 2]>,
    ) -> Option<[(Self, Result<Self, ()>); 2]> {
        a
    }
}

fn main() {}
