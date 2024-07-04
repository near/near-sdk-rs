#![allow(deprecated)]

use near_sdk::borsh::{BorshDeserialize, BorshSerialize};
use near_sdk::{near, store, PanicOnDefault};
use Collection::*;

#[derive(BorshSerialize, BorshDeserialize, Ord, PartialOrd, Eq, PartialEq, Clone)]
#[borsh(crate = "near_sdk::borsh")]
pub struct Insertable {
    pub index: u32,
    pub data: String,
    pub is_valid: bool,
}

#[near(contract_state)]
#[derive(PanicOnDefault)]
// This contract is designed for testing all of the `store` collections.
pub struct StoreContract {
    pub iterable_set: store::IterableSet<Insertable>,
    pub iterable_map: store::IterableMap<u32, Insertable>,
    pub unordered_set: store::UnorderedSet<Insertable>,
    pub unordered_map: store::UnorderedMap<u32, Insertable>,
    pub tree_map: store::TreeMap<u32, Insertable>,
    pub lookup_map: store::LookupMap<u32, Insertable>,
    pub lookup_set: store::LookupSet<Insertable>,
    pub vec: store::Vector<Insertable>,
}

#[near(serializers=[json])]
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

    fn insertable(&self) -> Insertable {
        Insertable { index: 0, data: "scatter cinnamon wheel useless please rough situate iron eager noise try evolve runway neglect onion".to_string(), is_valid: true }
    }

    #[payable]
    pub fn insert(&mut self, col: Collection, index_offset: usize, iterations: usize) {
        let mut insertable = self.insertable();
        for iter in 0..=iterations {
            insertable.index = iter as u32;
            insertable.index += index_offset as u32;
            self.insert_op(&col, insertable.clone())
        }
    }

    #[payable]
    pub fn remove(&mut self, col: Collection, iterations: usize) {
        let mut insertable = self.insertable();
        for iter in 0..=iterations {
            insertable.index = iter as u32;
            self.remove_op(&col, &insertable)
        }
    }

    #[payable]
    pub fn contains(&mut self, col: Collection, repeat: usize, iterations: usize) {
        let mut insertable = self.insertable();
        for iter in 0..=iterations {
            insertable.index = iter as u32;
            for _ in 0..repeat {
                self.contains_op(&col, &insertable)
            }
        }
    }

    #[payable]
    pub fn iter(&mut self, col: Collection, repeat: usize, iterations: usize) {
        for _ in 0..=iterations {
            for _ in 0..repeat {
                self.iter_op(&col, iterations)
            }
        }
    }

    #[payable]
    pub fn nth(&mut self, col: Collection, repeat: usize, iterations: usize) {
        for iter in 0..=iterations {
            for _ in 0..repeat {
                self.nth_op(&col, iter)
            }
        }
    }

    fn insert_op(&mut self, col: &Collection, val: Insertable) {
        match col {
            IterableSet => {
                self.iterable_set.insert(val);
            }
            IterableMap => {
                self.iterable_map.insert(val.index, val);
            }
            UnorderedMap => {
                self.unordered_map.insert(val.index, val);
            }
            UnorderedSet => {
                self.unordered_set.insert(val);
            }
            LookupMap => {
                self.lookup_map.insert(val.index, val);
            }
            LookupSet => {
                self.lookup_set.insert(val);
            }
            TreeMap => {
                self.tree_map.insert(val.index, val);
            }
            Vector => {
                self.vec.push(val);
            }
        };
    }

    fn remove_op(&mut self, col: &Collection, val: &Insertable) {
        match col {
            IterableSet => {
                self.iterable_set.remove(&val);
            }
            IterableMap => {
                self.iterable_map.remove(&val.index);
            }
            UnorderedMap => {
                self.unordered_map.remove(&val.index);
            }
            UnorderedSet => {
                self.unordered_set.remove(&val);
            }
            LookupMap => {
                self.lookup_map.remove(&val.index);
            }
            LookupSet => {
                self.lookup_set.remove(&val);
            }
            TreeMap => {
                self.tree_map.remove(&val.index);
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

    fn contains_op(&mut self, col: &Collection, val: &Insertable) {
        match col {
            IterableSet => self.iterable_set.contains(val),
            IterableMap => self.iterable_map.contains_key(&val.index),
            UnorderedMap => self.unordered_map.contains_key(&val.index),
            UnorderedSet => self.unordered_set.contains(val),
            LookupMap => self.lookup_map.contains_key(&val.index),
            LookupSet => self.lookup_set.contains(val),
            TreeMap => self.tree_map.contains_key(&val.index),
            // no contains method
            Vector => unimplemented!(),
        };
    }

    fn iter_op(&mut self, col: &Collection, take: usize) {
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
            LookupMap => unimplemented!(),
            LookupSet => unimplemented!(),
        };
    }

    fn nth_op(&mut self, col: &Collection, element_idx: usize) {
        match col {
            IterableSet => {
                self.iterable_set.iter().nth(element_idx);
            }
            IterableMap => {
                self.iterable_map.iter().nth(element_idx);
            }
            UnorderedMap => {
                self.unordered_map.iter().nth(element_idx);
            }
            UnorderedSet => {
                self.unordered_set.iter().nth(element_idx);
            }
            TreeMap => {
                self.tree_map.iter().nth(element_idx);
            }
            Vector => {
                self.vec.iter().nth(element_idx);
            }
            // Lookup* collections are not iterable.
            LookupMap => unimplemented!(),
            LookupSet => unimplemented!(),
        };
    }
}
