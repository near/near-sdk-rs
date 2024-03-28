//! Complex smart contract.

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
    Eq, PartialEq, Hash, PartialOrd,
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
    pub fn insert(&mut self, key: TypeA, value: TypeB) -> Option<TypeB> {
        self.map.insert(key, value)
    }
}

fn main() {}
