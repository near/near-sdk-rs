use std::marker::PhantomData;
use borsh::{BorshDeserialize, BorshSerialize};

use crate::collections::{append, serialize, deserialize, Heap, append_slice};
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
    key_prefix: Vec<u8>,
    keys: Heap<K>,
    _values: PhantomData<V>,
}

impl<K, V> HeapMap<K, V>
    where
        K: Ord + BorshSerialize + BorshDeserialize,
        V: BorshSerialize + BorshDeserialize,
{
    pub fn new(id: Vec<u8>) -> Self {
        let key_prefix = append(&id, b'i');
        let elements_prefix = append(&id, b'e');

        Self { key_prefix, keys: Heap::new(elements_prefix), _values: PhantomData }
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
        env::storage_read(&append_slice(&self.key_prefix, &serialize(key)))
            .map(|raw| deserialize(&raw))
    }

    pub fn remove(&mut self, key: &K) -> Option<V> {
        let opt = self.get(key);
        self.keys.remove(key);
        env::storage_remove(&append_slice(&self.key_prefix, &serialize(key)));
        opt
    }

    pub fn insert(&mut self, key: &K, value: &V) -> Option<V> {
        let opt = self.get(key);
        self.keys.insert(key);
        env::storage_write(
            &append_slice(&self.key_prefix, &serialize(key)),
            &serialize(value));
        opt
    }

    // Note: iterator mutates underlying indexes when sorts the keys.
    pub fn keys<'a>(&'a mut self) -> impl Iterator<Item = K> + 'a {
        self.keys.iter()
    }

    // Note: mutable borrow is required for iterator.
    pub fn entries<'a>(&'a mut self) -> impl Iterator<Item = (K, V)> + 'a {
        let prefix = self.key_prefix.clone();
        self.keys.iter()
            .map(move |key| {
                let raw_key = append_slice(&prefix, &serialize(&key));
                let raw_val: Vec<u8> = env::storage_read(&raw_key).unwrap();
                let val = deserialize(&raw_val);
                (key, val)
            })
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::test_env;

    #[test]
    fn test_empty() {
        test_env::setup();

        let mut map: HeapMap<u32, u32> = HeapMap::new(vec![b't']);
        assert_eq!(map.len(), 0);
        map.clear();
    }

    #[test]
    fn test_insert() {
        test_env::setup();
        let mut map: HeapMap<u32, u32> = HeapMap::new(vec![b't']);

        let key: u32 = 42;
        let val: u32 = 2020;

        assert!(!map.contains_key(&key));
        map.insert(&key, &val);
        assert!(map.contains_key(&key));

        assert_eq!(map.get(&key), Some(val));
        map.clear();
    }

    #[test]
    fn test_remove() {
        test_env::setup();
        let mut map: HeapMap<u32, u32> = HeapMap::new(vec![b't']);

        let key: u32 = 42;
        let val: u32 = 2020;

        assert!(!map.contains_key(&key));
        map.insert(&key, &val);
        map.remove(&key);

        assert_eq!(map.get(&key), None);
        map.clear();
    }

    #[test]
    fn test_keys() {
        test_env::setup();
        let mut map: HeapMap<u32, u32> = HeapMap::new(vec![b't']);

        let tuples: Vec<(u32, u32)> = vec![
            (105, 999),
            (104, 998),
            (103, 997),
            (102, 996),
            (101, 995),
        ];

        for (k, v) in &tuples {
            map.insert(k, v);
        }

        let mut expected: Vec<u32> = tuples.into_iter().map(|e| e.0).collect();
        expected.sort();

        let keys = map.keys().collect::<Vec<u32>>();
        assert_eq!(keys, expected);

        map.clear();
    }

    #[test]
    fn test_entries() {
        test_env::setup();
        let mut map: HeapMap<u32, u32> = HeapMap::new(vec![b't']);

        let mut tuples: Vec<(u32, u32)> = vec![
            (155, 999),
            (144, 998),
            (133, 997),
            (122, 996),
            (111, 995),
        ];

        for (k, v) in &tuples {
            map.insert(k, v);
        }

        tuples.sort_by_key(|e| e.0);

        let entries = map.entries().collect::<Vec<(u32, u32)>>();
        assert_eq!(entries, tuples);

        map.clear();
    }
}
