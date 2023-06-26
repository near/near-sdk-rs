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
    pub fn plain_ret() -> Self {
        unimplemented!()
    }
}

fn main() {}
