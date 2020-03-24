//! Complex smart contract.

use near_sdk::near_bindgen;
use borsh::{BorshDeserialize, BorshSerialize};
use std::collections::HashMap;
use serde::{Serialize, Deserialize};

#[derive(BorshDeserialize, BorshSerialize, Eq, PartialEq, Hash, PartialOrd, Serialize, Deserialize)]
enum TypeA {
    Var1,
    Var2
}

#[derive(BorshDeserialize, BorshSerialize, Eq, PartialEq, Hash, PartialOrd, Serialize, Deserialize)]
enum TypeB {
    Var1,
    Var2
}

#[near_bindgen]
#[derive(Default, BorshDeserialize, BorshSerialize)]
struct Storage {
    map: HashMap<TypeA, TypeB>
}

#[near_bindgen]
impl Storage {
    pub fn insert(&mut self, key: TypeA, value: TypeB) -> Option<TypeB> {
        self.map.insert(key, value)
    }
}

fn main() {}
