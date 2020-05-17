use std::marker::PhantomData;
use borsh::{BorshDeserialize, BorshSerialize};

use crate::collections::{append, serialize, deserialize, Heap};
use crate::env;

/// HeapMap allows iterating over keys and entries based on natural key ordering.
///
/// Runtime complexity (worst case):
///   - `contains_key`: O(1)
///   - `get`: O(1)
///   - `insert`: O(log(N))
///   - `remove`: O(log(N))
///   - `keys` (iterator): O(Nlog(N))
///   - `entries` (iterator): O(Nlog(N))
#[derive(BorshSerialize, BorshDeserialize)]
pub struct HeapMap<K, V> {
    entry_index_prefix: Vec<u8>,
    keys: Heap<K>,
    _values: PhantomData<V>,
}

impl<K, V> HeapMap<K, V>
    where
        K: Ord + BorshSerialize + BorshDeserialize,
        V: BorshSerialize + BorshDeserialize,
{
    pub fn new(id: Vec<u8>) -> Self {
        let entry_index_prefix = append(&id, b'i');
        let elements_prefix = append(&id, b'e');

        Self { entry_index_prefix, keys: Heap::new(elements_prefix), _values: PhantomData }
    }

    pub fn len(&self) -> u64 {
        self.keys.len()
    }

    pub fn clear(&mut self) {
        self.keys.clear();
    }

    pub fn contains_key(&self, key: &K) -> bool {
        self.keys.lookup(key).is_some()
    }

    pub fn get(&self, key: &K) -> Option<V> {
        env::storage_read(&serialize(key))
            .map(|raw| deserialize(&raw))
    }

    // TODO remove
    // TODO insert
    // TODO keys (iterator)
    // TODO entries (iterator)
}
