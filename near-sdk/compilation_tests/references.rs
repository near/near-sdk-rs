//! Method with non-deserializable argument type.

use borsh::{BorshDeserialize, BorshSerialize};
use near_sdk::near_bindgen;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(
    BorshDeserialize, BorshSerialize, Eq, PartialEq, Hash, PartialOrd, Serialize, Deserialize,
)]
pub enum TypeA {
    Var1,
    Var2,
}

#[derive(
    BorshDeserialize, BorshSerialize, Eq, PartialEq, Hash, PartialOrd, Serialize, Deserialize,
)]
pub enum TypeB {
    Var1,
    Var2,
}

#[near_bindgen]
#[derive(Default, BorshDeserialize, BorshSerialize)]
struct Storage {
    map: HashMap<TypeA, TypeB>,
}

#[near_bindgen]
impl Storage {
    pub fn get(&self, key: &TypeA) -> &TypeB {
        self.map.get(key).unwrap()
    }
}

fn main() {}
