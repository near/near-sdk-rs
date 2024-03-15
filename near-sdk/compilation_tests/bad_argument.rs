//! Method with non-deserializable argument type.

use near_sdk::near;
use std::collections::HashMap;

#[derive(Eq, PartialEq, Hash, PartialOrd, Ord)]
#[near(serializers=[borsh, json])]
enum TypeA {
    Var1,
    Var2
}

#[derive(Eq, PartialEq, Hash, PartialOrd)]
#[near(serializers=[borsh, json])]
enum TypeB {
    Var1,
    Var2
}

#[near(contract_state)]
#[derive(Default)]
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
