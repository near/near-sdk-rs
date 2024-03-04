//! Method signature uses Self.

use near_sdk::near_bindgen;
use serde::{Deserialize, Serialize};

#[near(contract_state)]
#[derive(Default, Serialize, Deserialize)]
pub struct Ident {
    value: u32,
}

#[near]
impl Ident {
    pub fn plain_ret() -> Self {
        unimplemented!()
    }
}

fn main() {}
