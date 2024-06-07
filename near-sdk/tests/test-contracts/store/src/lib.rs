#![allow(deprecated)]

use near_sdk::borsh::{BorshDeserialize, BorshSerialize};
use near_sdk::serde::{Deserialize, Serialize};
use near_sdk::sys::panic;
use near_sdk::{near, near_bindgen, store, PanicOnDefault};
use Collection::*;

#[near(contract_state)]
#[derive(PanicOnDefault)]
// This contract is designed for testing all of the `store` collections.
pub struct StoreContract {
    pub iterable_set: store::IterableSet<u32>,
    pub iterable_map: store::IterableMap<u32, u32>,
    pub unordered_set: store::UnorderedSet<u32>,
    pub unordered_map: store::UnorderedMap<u32, u32>,
    pub tree_map: store::TreeMap<u32, u32>,
    pub lookup_map: store::LookupMap<u32, u32>,
    pub lookup_set: store::LookupSet<u32>,
    pub vec: store::Vector<u32>,
}

#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub enum Collection {
    IterableSet,
    IterableMap,
    UnorderedSet,
    UnorderedMap,
    LookupMap,
    LookupSet,
    TreeMap,
    Vector,
}

#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub enum Op {
    Insert(u32),
    Remove(u32),
    Flush,
    Contains(u32),
    Iter(usize),
}

#[near]
impl StoreContract {
    #[init]
    pub fn new() -> Self {
        let vec = store::Vector::new(b"1");
        let iterable_set = store::IterableSet::new(b"2");
        let iterable_map = store::IterableMap::new(b"3");
        let unordered_set = store::UnorderedSet::new(b"4");
        let unordered_map = store::UnorderedMap::new(b"5");
        let tree_map = store::TreeMap::new(b"6");
        let lookup_map = store::LookupMap::new(b"7");
        let lookup_set = store::LookupSet::new(b"8");

        Self {
            vec,
            iterable_set,
            iterable_map,
            unordered_set,
            unordered_map,
            tree_map,
            lookup_map,
            lookup_set,
        }
    }

    #[payable]
    pub fn exec(&mut self, col: Collection, op: Op) {
        match op {
            Op::Insert(val) => self.insert(col, val),
            Op::Remove(val) => self.remove(col, val),
            Op::Flush => self.flush(col),
            Op::Contains(val) => self.contains(col, val),
            Op::Iter(take) => self.iter(col, take),
        }
    }

    fn insert(&mut self, col: Collection, val: u32) {
        match col {
            IterableSet => {
                self.iterable_set.insert(val);
            }
            IterableMap => {
                self.iterable_map.insert(val, val);
            }
            UnorderedMap => {
                self.unordered_map.insert(val, val);
            }
            UnorderedSet => {
                self.unordered_set.insert(val);
            }
            LookupMap => {
                self.lookup_map.insert(val, val);
            }
            LookupSet => {
                self.lookup_set.insert(val);
            }
            TreeMap => {
                self.tree_map.insert(val, val);
            }
            Vector => {
                self.vec.push(val);
            }
        };
    }

    fn remove(&mut self, col: Collection, val: u32) {
        match col {
            IterableSet => {
                self.iterable_set.remove(&val);
            }
            IterableMap => {
                self.iterable_map.remove(&val);
            }
            UnorderedMap => {
                self.unordered_map.remove(&val);
            }
            UnorderedSet => {
                self.unordered_set.remove(&val);
            }
            LookupMap => {
                self.lookup_map.remove(&val);
            }
            LookupSet => {
                self.lookup_set.remove(&val);
            }
            TreeMap => {
                self.tree_map.remove(&val);
            }
            Vector => {
                if self.vec.is_empty() {
                    return;
                }
                // Take the opportunity to take swap and pop.
                self.vec.swap_remove(self.vec.len() - 1);
            }
        };
    }

    fn flush(&mut self, col: Collection) {
        match col {
            IterableSet => self.iterable_set.flush(),
            IterableMap => self.iterable_map.flush(),
            UnorderedMap => self.unordered_map.flush(),
            UnorderedSet => self.unordered_set.flush(),
            LookupMap => self.lookup_map.flush(),
            TreeMap => self.tree_map.flush(),
            Vector => self.vec.flush(),
            // No flush method.
            LookupSet => unimplemented!(),
        };
    }

    fn contains(&mut self, col: Collection, val: u32) {
        match col {
            IterableSet => self.iterable_set.contains(&val),
            IterableMap => self.iterable_map.contains_key(&val),
            UnorderedMap => self.unordered_map.contains_key(&val),
            UnorderedSet => self.unordered_set.contains(&val),
            LookupMap => self.lookup_map.contains_key(&val),
            LookupSet => self.lookup_set.contains(&val),
            TreeMap => self.tree_map.contains_key(&val),
            // no contains method
            Vector => unimplemented!(),
        };
    }

    fn iter(&mut self, col: Collection, take: usize) {
        match col {
            IterableSet => {
                let mut iter = self.iterable_set.iter();
                for _ in 0..take {
                    iter.next();
                }
            }
            IterableMap => {
                let mut iter = self.iterable_map.iter();
                for _ in 0..take {
                    iter.next();
                }
            }
            UnorderedMap => {
                let mut iter = self.unordered_map.iter();
                for _ in 0..take {
                    iter.next();
                }
            }
            UnorderedSet => {
                let mut iter = self.unordered_set.iter();
                for _ in 0..take {
                    iter.next();
                }
            }
            TreeMap => {
                let mut iter = self.tree_map.iter();
                for _ in 0..take {
                    iter.next();
                }
            }
            Vector => {
                let mut iter = self.vec.iter();
                for _ in 0..take {
                    iter.next();
                }
            }
            // Lookup* collections are not iterable.
            LookupMap => {}
            LookupSet => {}
        };
    }
}
