//! A persistent set without iterators. Unlike `near_sdk::collections::LookupSet` this set
//! doesn't store values separately in a vector, so it can't iterate over the values. But it
//! makes this implementation more efficient in the number of reads and writes.
use std::marker::PhantomData;

use borsh::{BorshDeserialize, BorshSerialize};

use crate::collections::append_slice;
use crate::env;

const ERR_ELEMENT_SERIALIZATION: &[u8] = b"Cannot serialize element with Borsh";

/// An non-iterable implementation of a set that stores its content directly on the trie.
#[derive(BorshSerialize, BorshDeserialize)]
pub struct LookupSet<T> {
    element_prefix: Vec<u8>,
    #[borsh_skip]
    el: PhantomData<T>,
}

impl<T> LookupSet<T> {
    /// Create a new map. Use `element_prefix` as a unique prefix for trie keys.
    pub fn new(element_prefix: Vec<u8>) -> Self {
        Self { element_prefix, el: PhantomData }
    }

    fn raw_element_to_storage_key(&self, element_raw: &[u8]) -> Vec<u8> {
        append_slice(&self.element_prefix, element_raw)
    }

    /// Returns `true` if the serialized key is present in the map.
    fn contains_raw(&self, element_raw: &[u8]) -> bool {
        let storage_key = self.raw_element_to_storage_key(element_raw);
        env::storage_has_key(&storage_key)
    }

    /// Inserts a serialized element into the set.
    /// If the set did not have this value present, `true` is returned.
    /// If the set did have this value present, `false` is returned.
    pub fn insert_raw(&mut self, element_raw: &[u8]) -> bool {
        let storage_key = self.raw_element_to_storage_key(element_raw);
        !env::storage_write(&storage_key, b"")
    }

    /// Removes a serialized element from the set.
    /// Returns true if the element was present in the set.
    pub fn remove_raw(&mut self, element_raw: &[u8]) -> bool {
        let storage_key = self.raw_element_to_storage_key(element_raw);
        env::storage_remove(&storage_key)
    }
}

impl<T> LookupSet<T>
where
    T: BorshSerialize,
{
    fn serialize_element(element: &T) -> Vec<u8> {
        match element.try_to_vec() {
            Ok(x) => x,
            Err(_) => env::panic(ERR_ELEMENT_SERIALIZATION),
        }
    }

    /// Returns true if the set contains an element.
    pub fn contains(&self, element: &T) -> bool {
        self.contains_raw(&Self::serialize_element(element))
    }

    /// Removes a value from the set. Returns whether the value was present in the set.
    pub fn remove(&mut self, element: &T) -> bool {
        self.remove_raw(&Self::serialize_element(element))
    }

    /// Adds a value to the set.
    /// If the set did not have this value present, `true` is returned.
    /// If the set did have this value present, `false` is returned.
    pub fn insert(&mut self, element: &T) -> bool {
        self.insert_raw(&Self::serialize_element(element))
    }

    pub fn extend<IT: IntoIterator<Item = T>>(&mut self, iter: IT) {
        for el in iter {
            self.insert(&el);
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[cfg(test)]
mod tests {
    use crate::collections::LookupSet;
    use crate::test_utils::test_env;
    use rand::seq::SliceRandom;
    use rand::{Rng, SeedableRng};
    use std::collections::HashSet;

    #[test]
    pub fn test_insert() {
        test_env::setup();
        let mut set = LookupSet::new(b"s".to_vec());
        let mut rng = rand_xorshift::XorShiftRng::seed_from_u64(0);
        for _ in 0..500 {
            let key = rng.gen::<u64>();
            set.insert(&key);
        }
    }

    #[test]
    pub fn test_insert_remove() {
        test_env::setup();
        let mut set = LookupSet::new(b"s".to_vec());
        let mut rng = rand_xorshift::XorShiftRng::seed_from_u64(1);
        let mut keys = vec![];
        for _ in 0..100 {
            let key = rng.gen::<u64>();
            keys.push(key);
            set.insert(&key);
        }
        keys.shuffle(&mut rng);
        for key in keys {
            assert!(set.remove(&key));
        }
    }

    #[test]
    pub fn test_remove_last_reinsert() {
        test_env::setup();
        let mut set = LookupSet::new(b"s".to_vec());
        let key1 = 1u64;
        set.insert(&key1);
        let key2 = 2u64;
        set.insert(&key2);

        let actual = set.remove(&key2);
        assert!(actual);

        let actual_reinsert = set.insert(&key2);
        assert!(actual_reinsert);
    }

    #[test]
    pub fn test_insert_override_remove() {
        test_env::setup();
        let mut set = LookupSet::new(b"s".to_vec());
        let mut rng = rand_xorshift::XorShiftRng::seed_from_u64(2);
        let mut keys = vec![];
        for _ in 0..100 {
            let key = rng.gen::<u64>();
            keys.push(key);
            set.insert(&key);
        }
        keys.shuffle(&mut rng);
        for key in &keys {
            assert!(!set.insert(key));
        }
        keys.shuffle(&mut rng);
        for key in keys {
            assert!(set.remove(&key));
        }
    }

    #[test]
    pub fn test_contains_non_existent() {
        test_env::setup();
        let mut set = LookupSet::new(b"s".to_vec());
        let mut rng = rand_xorshift::XorShiftRng::seed_from_u64(3);
        let mut set_tmp = HashSet::new();
        for _ in 0..500 {
            let key = rng.gen::<u64>() % 20_000;
            set_tmp.insert(key);
            set.insert(&key);
        }
        for _ in 0..500 {
            let key = rng.gen::<u64>() % 20_000;
            assert_eq!(set.contains(&key), set_tmp.contains(&key));
        }
    }

    #[test]
    pub fn test_extend() {
        test_env::setup();
        let mut set = LookupSet::new(b"s".to_vec());
        let mut rng = rand_xorshift::XorShiftRng::seed_from_u64(4);
        let mut keys = HashSet::new();
        for _ in 0..100 {
            let key = rng.gen::<u64>();
            keys.insert(key);
            set.insert(&key);
        }
        for _ in 0..10 {
            let mut tmp = vec![];
            for _ in 0..=(rng.gen::<u64>() % 20 + 1) {
                let key = rng.gen::<u64>();
                tmp.push(key);
            }
            keys.extend(tmp.iter().cloned());
            set.extend(tmp.iter().cloned());
        }

        for key in keys {
            assert!(set.contains(&key));
        }
    }
}
