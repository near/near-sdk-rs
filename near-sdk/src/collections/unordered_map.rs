//! A map implemented on a trie. Unlike `std::collections::HashMap` the keys in this map are not
//! hashed but are instead serialized.
use crate::collections::{append, append_slice, Vector};
use crate::env;
use borsh::{BorshDeserialize, BorshSerialize};
use std::mem::size_of;

const ERR_INCONSISTENT_STATE: &[u8] = b"The collection is an inconsistent state. Did previous smart contract execution terminate unexpectedly?";
const ERR_KEY_SERIALIZATION: &[u8] = b"Cannot serialize key with Borsh";
const ERR_VALUE_DESERIALIZATION: &[u8] = b"Cannot deserialize value with Borsh";
const ERR_VALUE_SERIALIZATION: &[u8] = b"Cannot serialize value with Borsh";

/// An iterable implementation of a map that stores its content directly on the trie.
#[derive(BorshSerialize, BorshDeserialize)]
pub struct UnorderedMap<K, V> {
    key_index_prefix: Vec<u8>,
    keys: Vector<K>,
    values: Vector<V>,
}

impl<K, V> UnorderedMap<K, V> {
    /// Returns the number of elements in the map, also referred to as its size.
    pub fn len(&self) -> u64 {
        let keys_len = self.keys.len();
        let values_len = self.values.len();
        if keys_len != values_len {
            env::panic(ERR_INCONSISTENT_STATE)
        } else {
            keys_len
        }
    }

    /// Returns `true` if the map contains no elements.
    pub fn is_empty(&self) -> bool {
        let keys_is_empty = self.keys.is_empty();
        let values_is_empty = self.values.is_empty();
        if keys_is_empty != values_is_empty {
            env::panic(ERR_INCONSISTENT_STATE)
        } else {
            keys_is_empty
        }
    }

    /// Create new map with zero elements. Use `id` as a unique identifier.
    pub fn new(id: Vec<u8>) -> Self {
        let key_index_prefix = append(&id, b'i');
        let index_key_id = append(&id, b'k');
        let index_value_id = append(&id, b'v');

        Self {
            key_index_prefix,
            keys: Vector::new(index_key_id),
            values: Vector::new(index_value_id),
        }
    }

    fn serialize_index(index: u64) -> [u8; size_of::<u64>()] {
        index.to_le_bytes()
    }

    fn deserialize_index(raw_index: &[u8]) -> u64 {
        let mut result = [0u8; size_of::<u64>()];
        result.copy_from_slice(raw_index);
        u64::from_le_bytes(result)
    }

    fn raw_key_to_index_lookup(&self, raw_key: &[u8]) -> Vec<u8> {
        append_slice(&self.key_index_prefix, raw_key)
    }

    /// Returns an index of the given raw key.
    fn get_index_raw(&self, key_raw: &[u8]) -> Option<u64> {
        let index_lookup = self.raw_key_to_index_lookup(key_raw);
        env::storage_read(&index_lookup).map(|raw_index| Self::deserialize_index(&raw_index))
    }

    /// Returns the serialized value corresponding to the serialized key.
    fn get_raw(&self, key_raw: &[u8]) -> Option<Vec<u8>> {
        self.get_index_raw(key_raw).map(|index| match self.values.get_raw(index) {
            Some(x) => x,
            None => env::panic(ERR_INCONSISTENT_STATE),
        })
    }

    /// Inserts a serialized key-value pair into the map.
    /// If the map did not have this key present, `None` is returned. Otherwise returns
    /// a serialized value. Note, the keys that have the same hash value are undistinguished by
    /// the implementation.
    pub fn insert_raw(&mut self, key_raw: &[u8], value_raw: &[u8]) -> Option<Vec<u8>> {
        let index_lookup = self.raw_key_to_index_lookup(key_raw);
        match env::storage_read(&index_lookup) {
            Some(index_raw) => {
                // The element already exists.
                let index = Self::deserialize_index(&index_raw);
                Some(self.values.replace_raw(index, value_raw))
            }
            None => {
                // The element does not exist yet.
                let next_index = self.len();
                let next_index_raw = Self::serialize_index(next_index);
                env::storage_write(&index_lookup, &next_index_raw);
                self.keys.push_raw(key_raw);
                self.values.push_raw(value_raw);
                None
            }
        }
    }

    /// Removes a serialized key from the map, returning the serialized value at the key if the key
    /// was previously in the map.
    pub fn remove_raw(&mut self, key_raw: &[u8]) -> Option<Vec<u8>> {
        let index_lookup = self.raw_key_to_index_lookup(key_raw);
        match env::storage_read(&index_lookup) {
            Some(index_raw) => {
                if self.len() == 1 {
                    // If there is only one element then swap remove simply removes it without
                    // swapping with the last element.
                    env::storage_remove(&index_lookup);
                } else {
                    // If there is more than one element then swap remove swaps it with the last
                    // element.
                    let last_key_raw = match self.keys.get_raw(self.len() - 1) {
                        Some(x) => x,
                        None => env::panic(ERR_INCONSISTENT_STATE),
                    };
                    env::storage_remove(&index_lookup);
                    // If the removed element was the last element from keys, then we don't need to
                    // reinsert the lookup back.
                    if last_key_raw != key_raw {
                        let last_lookup_key = self.raw_key_to_index_lookup(&last_key_raw);
                        env::storage_write(&last_lookup_key, &index_raw);
                    }
                }
                let index = Self::deserialize_index(&index_raw);
                self.keys.swap_remove_raw(index);
                Some(self.values.swap_remove_raw(index))
            }
            None => None,
        }
    }
}

impl<K, V> UnorderedMap<K, V>
where
    K: BorshSerialize + BorshDeserialize,
    V: BorshSerialize + BorshDeserialize,
{
    fn serialize_key(key: &K) -> Vec<u8> {
        match key.try_to_vec() {
            Ok(x) => x,
            Err(_) => env::panic(ERR_KEY_SERIALIZATION),
        }
    }

    fn deserialize_value(raw_value: &[u8]) -> V {
        match V::try_from_slice(&raw_value) {
            Ok(x) => x,
            Err(_) => env::panic(ERR_VALUE_DESERIALIZATION),
        }
    }

    fn serialize_value(value: &V) -> Vec<u8> {
        match value.try_to_vec() {
            Ok(x) => x,
            Err(_) => env::panic(ERR_VALUE_SERIALIZATION),
        }
    }

    /// Returns the value corresponding to the key.
    pub fn get(&self, key: &K) -> Option<V> {
        self.get_raw(&Self::serialize_key(key)).map(|value_raw| Self::deserialize_value(&value_raw))
    }

    /// Removes a key from the map, returning the value at the key if the key was previously in the
    /// map.
    pub fn remove(&mut self, key: &K) -> Option<V> {
        self.remove_raw(&Self::serialize_key(key))
            .map(|value_raw| Self::deserialize_value(&value_raw))
    }

    /// Inserts a key-value pair into the map.
    /// If the map did not have this key present, `None` is returned. Otherwise returns
    /// a value. Note, the keys that have the same hash value are undistinguished by
    /// the implementation.
    pub fn insert(&mut self, key: &K, value: &V) -> Option<V> {
        self.insert_raw(&Self::serialize_key(key), &Self::serialize_value(&value))
            .map(|value_raw| Self::deserialize_value(&value_raw))
    }

    /// Clears the map, removing all elements.
    pub fn clear(&mut self) {
        for raw_key in self.keys.iter_raw() {
            let index_lookup = self.raw_key_to_index_lookup(&raw_key);
            env::storage_remove(&index_lookup);
        }
        self.keys.clear();
        self.values.clear();
    }

    /// Copies elements into an `std::vec::Vec`.
    pub fn to_vec(&self) -> std::vec::Vec<(K, V)> {
        self.iter().collect()
    }

    /// An iterator visiting all keys. The iterator element type is `K`.
    pub fn keys<'a>(&'a self) -> impl Iterator<Item = K> + 'a {
        self.keys.iter()
    }

    /// An iterator visiting all values. The iterator element type is `V`.
    pub fn values<'a>(&'a self) -> impl Iterator<Item = V> + 'a {
        self.values.iter()
    }

    /// Iterate over deserialized keys and values.
    pub fn iter<'a>(&'a self) -> impl Iterator<Item = (K, V)> + 'a {
        self.keys.iter().zip(self.values.iter())
    }

    pub fn extend<IT: IntoIterator<Item = (K, V)>>(&mut self, iter: IT) {
        for (el_key, el_value) in iter {
            self.insert(&el_key, &el_value);
        }
    }

    /// Returns a view of keys as a vector.
    /// It's sometimes useful to have random access to the keys.
    pub fn keys_as_vector(&self) -> &Vector<K> {
        &self.keys
    }

    /// Returns a view of values as a vector.
    /// It's sometimes useful to have random access to the values.
    pub fn values_as_vector(&self) -> &Vector<V> {
        &self.values
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[cfg(test)]
mod tests {
    use crate::collections::UnorderedMap;
    use crate::test_utils::test_env;
    use rand::seq::SliceRandom;
    use rand::{Rng, SeedableRng};
    use std::collections::{HashMap, HashSet};
    use std::iter::FromIterator;

    #[test]
    pub fn test_insert() {
        test_env::setup();
        let mut map = UnorderedMap::new(b"m".to_vec());
        let mut rng = rand_xorshift::XorShiftRng::seed_from_u64(0);
        for _ in 0..500 {
            let key = rng.gen::<u64>();
            let value = rng.gen::<u64>();
            map.insert(&key, &value);
        }
    }

    #[test]
    pub fn test_insert_remove() {
        test_env::setup();
        let mut map = UnorderedMap::new(b"m".to_vec());
        let mut rng = rand_xorshift::XorShiftRng::seed_from_u64(1);
        let mut keys = vec![];
        let mut key_to_value = HashMap::new();
        for _ in 0..100 {
            let key = rng.gen::<u64>();
            let value = rng.gen::<u64>();
            keys.push(key);
            key_to_value.insert(key, value);
            map.insert(&key, &value);
        }
        keys.shuffle(&mut rng);
        for key in keys {
            let actual = map.remove(&key).unwrap();
            assert_eq!(actual, key_to_value[&key]);
        }
    }

    #[test]
    pub fn test_remove_last_reinsert() {
        test_env::setup();
        let mut map = UnorderedMap::new(b"m".to_vec());
        let key1 = 1u64;
        let value1 = 2u64;
        map.insert(&key1, &value1);
        let key2 = 3u64;
        let value2 = 4u64;
        map.insert(&key2, &value2);

        let actual_value2 = map.remove(&key2).unwrap();
        assert_eq!(actual_value2, value2);

        let actual_insert_value2 = map.insert(&key2, &value2);
        assert_eq!(actual_insert_value2, None);
    }

    #[test]
    pub fn test_insert_override_remove() {
        test_env::setup();
        let mut map = UnorderedMap::new(b"m".to_vec());
        let mut rng = rand_xorshift::XorShiftRng::seed_from_u64(2);
        let mut keys = vec![];
        let mut key_to_value = HashMap::new();
        for _ in 0..100 {
            let key = rng.gen::<u64>();
            let value = rng.gen::<u64>();
            keys.push(key);
            key_to_value.insert(key, value);
            map.insert(&key, &value);
        }
        keys.shuffle(&mut rng);
        for key in &keys {
            let value = rng.gen::<u64>();
            let actual = map.insert(key, &value).unwrap();
            assert_eq!(actual, key_to_value[key]);
            key_to_value.insert(*key, value);
        }
        keys.shuffle(&mut rng);
        for key in keys {
            let actual = map.remove(&key).unwrap();
            assert_eq!(actual, key_to_value[&key]);
        }
    }

    #[test]
    pub fn test_get_non_existent() {
        test_env::setup();
        let mut map = UnorderedMap::new(b"m".to_vec());
        let mut rng = rand_xorshift::XorShiftRng::seed_from_u64(3);
        let mut key_to_value = HashMap::new();
        for _ in 0..500 {
            let key = rng.gen::<u64>() % 20_000;
            let value = rng.gen::<u64>();
            key_to_value.insert(key, value);
            map.insert(&key, &value);
        }
        for _ in 0..500 {
            let key = rng.gen::<u64>() % 20_000;
            assert_eq!(map.get(&key), key_to_value.get(&key).cloned());
        }
    }

    #[test]
    pub fn test_to_vec() {
        test_env::setup();
        let mut map = UnorderedMap::new(b"m".to_vec());
        let mut rng = rand_xorshift::XorShiftRng::seed_from_u64(4);
        let mut key_to_value = HashMap::new();
        for _ in 0..400 {
            let key = rng.gen::<u64>();
            let value = rng.gen::<u64>();
            key_to_value.insert(key, value);
            map.insert(&key, &value);
        }
        let actual = HashMap::from_iter(map.to_vec());
        assert_eq!(actual, key_to_value);
    }

    #[test]
    pub fn test_clear() {
        test_env::setup();
        let mut map = UnorderedMap::new(b"m".to_vec());
        let mut rng = rand_xorshift::XorShiftRng::seed_from_u64(5);
        for _ in 0..10 {
            for _ in 0..=(rng.gen::<u64>() % 20 + 1) {
                let key = rng.gen::<u64>();
                let value = rng.gen::<u64>();
                map.insert(&key, &value);
            }
            assert!(!map.to_vec().is_empty());
            map.clear();
            assert!(map.to_vec().is_empty());
        }
    }

    #[test]
    pub fn test_keys_values() {
        test_env::setup();
        let mut map = UnorderedMap::new(b"m".to_vec());
        let mut rng = rand_xorshift::XorShiftRng::seed_from_u64(4);
        let mut key_to_value = HashMap::new();
        for _ in 0..400 {
            let key = rng.gen::<u64>();
            let value = rng.gen::<u64>();
            key_to_value.insert(key, value);
            map.insert(&key, &value);
        }
        let actual: HashMap<u64, u64> = HashMap::from_iter(map.to_vec());
        assert_eq!(
            actual.keys().collect::<HashSet<_>>(),
            key_to_value.keys().collect::<HashSet<_>>()
        );
        assert_eq!(
            actual.values().collect::<HashSet<_>>(),
            key_to_value.values().collect::<HashSet<_>>()
        );
    }

    #[test]
    pub fn test_iter() {
        test_env::setup();
        let mut map = UnorderedMap::new(b"m".to_vec());
        let mut rng = rand_xorshift::XorShiftRng::seed_from_u64(4);
        let mut key_to_value = HashMap::new();
        for _ in 0..400 {
            let key = rng.gen::<u64>();
            let value = rng.gen::<u64>();
            key_to_value.insert(key, value);
            map.insert(&key, &value);
        }
        let actual: HashMap<u64, u64> = HashMap::from_iter(map.iter());
        assert_eq!(actual, key_to_value);
    }

    #[test]
    pub fn test_extend() {
        test_env::setup();
        let mut map = UnorderedMap::new(b"m".to_vec());
        let mut rng = rand_xorshift::XorShiftRng::seed_from_u64(4);
        let mut key_to_value = HashMap::new();
        for _ in 0..100 {
            let key = rng.gen::<u64>();
            let value = rng.gen::<u64>();
            key_to_value.insert(key, value);
            map.insert(&key, &value);
        }
        for _ in 0..10 {
            let mut tmp = vec![];
            for _ in 0..=(rng.gen::<u64>() % 20 + 1) {
                let key = rng.gen::<u64>();
                let value = rng.gen::<u64>();
                tmp.push((key, value));
            }
            key_to_value.extend(tmp.iter().cloned());
            map.extend(tmp.iter().cloned());
        }

        let actual: HashMap<u64, u64> = HashMap::from_iter(map.iter());
        assert_eq!(actual, key_to_value);
    }
}
