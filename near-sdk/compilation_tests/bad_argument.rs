//! Method with non-deserializable argument type.

use near_sdk::near_bindgen;
use borsh::{BorshDeserialize, BorshSerialize};
use std::collections::HashMap;
use serde::{Serialize, Deserialize};

#[derive(BorshSerialize, BorshDeserialize, Eq, PartialEq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
enum TypeA {
    Var1,
    Var2
}

#[derive(BorshSerialize, BorshDeserialize, Eq, PartialEq, Hash, PartialOrd, Serialize, Deserialize)]
enum TypeB {
    Var1,
    Var2
}

#[near(contract_state)]
#[derive(Default, BorshSerialize, BorshDeserialize)]
struct Storage {
    map: HashMap<TypeA, TypeB>
}

trait MyTrait {}

#[near]
impl Storage {
    pub fn insert(&mut self, key: TypeA, value: TypeB, t: impl MyTrait) -> Option<TypeB> {
        self.map.insert(key, value)
    }
}

fn main() {}
