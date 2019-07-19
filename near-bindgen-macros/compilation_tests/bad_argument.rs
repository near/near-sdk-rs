//! Method with non-deserializable argument type.

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

#[near_bindgen]
#[derive(Default, Serialize, Deserialize)]
struct Storage {
    map: HashMap<TypeA, TypeB>
}

trait MyTrait {}

#[near_bindgen]
impl Storage {
    pub fn insert(&mut self, key: TypeA, value: TypeB, t: impl MyTrait) -> Option<TypeB> {
        self.map.insert(key, value)
    }
}

fn main() {}
