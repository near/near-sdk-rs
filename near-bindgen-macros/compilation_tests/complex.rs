//! Complex smart contract.

#![feature(const_vec_new)]
use near_bindgen::near_bindgen;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Eq, PartialEq, Hash)]
enum TypeA {
    Var1,
    Var2
}

#[derive(Serialize, Deserialize, Eq, PartialEq, Hash)]
enum TypeB {
    Var1,
    Var2
}

#[derive(Default, Serialize, Deserialize)]
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
