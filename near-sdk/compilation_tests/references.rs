//! Method with non-deserializable argument type.

use near_sdk::near;
use std::collections::HashMap;

#[derive(
    Eq, PartialEq, Hash, PartialOrd, Ord,
)]
#[near(serializers=[borsh, json])]
pub enum TypeA {
    Var1,
    Var2,
}

#[derive(
    Eq, PartialEq, Hash, PartialOrd
)]
#[near(serializers=[borsh, json])]
pub enum TypeB {
    Var1,
    Var2,
}

#[near(contract_state)]
#[derive(Default)]
struct Storage {
    map: HashMap<TypeA, TypeB>,
}

#[near]
impl Storage {
    pub fn get(&self, key: &TypeA) -> &TypeB {
        self.map.get(key).unwrap()
    }
}

fn main() {}
