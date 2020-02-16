//! A map implemented on a trie. Unlike `std::collections::HashMap` the keys in this map are not
//! hashed but are instead serialized.
use crate::collections::{next_trie_id, Vector};
use crate::env;
use borsh::{BorshDeserialize, BorshSerialize};
use std::mem::size_of;

const ERR_INCONSISTENT_STATE: &str = "The collection is an inconsistent state. Did previous smart contract execution terminate unexpectedly?";
const ERR_KEY_SERIALIZATION: &str = "Cannot serialize key with Borsh";
const ERR_VALUE_DESERIALIZATION: &str = "Cannot deserialize value with Borsh";
const ERR_VALUE_SERIALIZATION: &str = "Cannot serialize value with Borsh";

/// An iterable implementation of a map that stores its content directly on the trie.
#[derive(BorshSerialize, BorshDeserialize)]
pub struct Map<K, V> {
    key_index_prefix: Vec<u8>,
    keys: Vector<K>,
    values: Vector<V>,
}

impl<K, V> Default for Map<K, V> {
    fn default() -> Self {
        Self::new(next_trie_id())
    }
}

impl<K, V> Map<K, V> {
    /// Returns the number of elements in the map, also referred to as its size.
    pub fn len(&self) -> u64 {
        let key_len = self.keys.len();
        let values_len = self.values.len();
        if key_len != values_len {
            env::panic(ERR_INCONSISTENT_STATE)
        } else {
            key_len
        }
    }

    /// Create new map with zero elements. Use `id` as a unique identifier.
    pub fn new(id: Vec<u8>) -> Self {
        let mut key_index_prefix = Vec::with_capacity(id.len() + 1);
        key_index_prefix.extend(&id);
        key_index_prefix.push(b'i');

        let mut index_key_id = Vec::with_capacity(id.len() + 1);
        index_key_id.extend(&id);
        index_key_id.push(b'k');

        let mut index_value_id = Vec::with_capacity(id.len() + 1);
        index_value_id.extend(&id);
        index_value_id.push(b'v');

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
        let mut res = Vec::with_capacity(self.key_index_prefix.len() + raw_key.len());
        res.extend_from_slice(&self.key_index_prefix);
        res.extend_from_slice(&raw_key);
        res
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
                env::storage_write(&index_lookup, &index_raw);
                let index = Self::deserialize_index(&index_raw);
                self.keys.replace_raw(index, key_raw);
                Some(self.values.replace_raw(index, value_raw))
            }
            None => {
                // The element does not exist yet.
                let next_index = self.len();
                let next_index_raw = Self::serialize_index(next_index);
                let key_lookup = self.raw_key_to_index_lookup(key_raw);
                env::storage_write(&key_lookup, &next_index_raw);
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
                    let last_lookup_key = self.raw_key_to_index_lookup(&last_key_raw);
                    env::storage_remove(&index_lookup);
                    env::storage_write(&last_lookup_key, &index_raw);
                }
                let index = Self::deserialize_index(&index_raw);
                self.keys.swap_remove_raw(index);
                Some(self.values.swap_remove_raw(index))
            }
            None => None,
        }
    }
}

impl<K, V> Map<K, V>
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
}

#[cfg(not(target_arch = "wasm32"))]
#[cfg(test)]
mod tests {
    use crate::collections::Map;
    use crate::{env, MockedBlockchain};
    use near_vm_logic::types::AccountId;
    use near_vm_logic::VMContext;
    use rand::seq::SliceRandom;
    use rand::{Rng, SeedableRng};
    use std::collections::{HashMap, HashSet};
    use std::iter::FromIterator;

    fn alice() -> AccountId {
        "alice.near".to_string()
    }
    fn bob() -> AccountId {
        "bob.near".to_string()
    }
    fn carol() -> AccountId {
        "carol.near".to_string()
    }

    fn set_env() {
        let context = VMContext {
            current_account_id: alice(),
            signer_account_id: bob(),
            signer_account_pk: vec![0, 1, 2],
            predecessor_account_id: carol(),
            input: vec![],
            block_index: 0,
            block_timestamp: 0,
            account_balance: 0,
            account_locked_balance: 0,
            storage_usage: 10u64.pow(6),
            attached_deposit: 0,
            prepaid_gas: 10u64.pow(18),
            random_seed: vec![0, 1, 2],
            is_view: false,
            output_data_receivers: vec![],
        };
        let storage = match env::take_blockchain_interface() {
            Some(mut bi) => bi.as_mut_mocked_blockchain().unwrap().take_storage(),
            None => Default::default(),
        };
        env::set_blockchain_interface(Box::new(MockedBlockchain::new(
            context,
            Default::default(),
            Default::default(),
            vec![],
            storage,
        )));
    }

    #[test]
    pub fn test_insert() {
        set_env();
        let mut map = Map::default();
        let mut rng = rand_xorshift::XorShiftRng::seed_from_u64(0);
        for _ in 0..1000 {
            let key = rng.gen::<u64>();
            let value = rng.gen::<u64>();
            map.insert(&key, &value);
        }
    }

    #[test]
    pub fn test_insert_remove() {
        set_env();
        let mut map = Map::default();
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
    pub fn test_insert_override_remove() {
        set_env();
        let mut map = Map::default();
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
        set_env();
        let mut map = Map::default();
        let mut rng = rand_xorshift::XorShiftRng::seed_from_u64(3);
        let mut key_to_value = HashMap::new();
        for _ in 0..1000 {
            let key = rng.gen::<u64>() % 20_000;
            let value = rng.gen::<u64>();
            key_to_value.insert(key, value);
            map.insert(&key, &value);
        }
        for _ in 0..1000 {
            let key = rng.gen::<u64>() % 20_000;
            assert_eq!(map.get(&key), key_to_value.get(&key).cloned());
        }
    }

    #[test]
    pub fn test_to_vec() {
        set_env();
        let mut map = Map::default();
        let mut rng = rand_xorshift::XorShiftRng::seed_from_u64(4);
        let mut key_to_value = HashMap::new();
        for _ in 0..1000 {
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
        set_env();
        let mut map = Map::default();
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
        set_env();
        let mut map = Map::default();
        let mut rng = rand_xorshift::XorShiftRng::seed_from_u64(4);
        let mut key_to_value = HashMap::new();
        for _ in 0..1000 {
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
        set_env();
        let mut map = Map::default();
        let mut rng = rand_xorshift::XorShiftRng::seed_from_u64(4);
        let mut key_to_value = HashMap::new();
        for _ in 0..1000 {
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
        set_env();
        let mut map = Map::default();
        let mut rng = rand_xorshift::XorShiftRng::seed_from_u64(4);
        let mut key_to_value = HashMap::new();
        for _ in 0..100 {
            let key = rng.gen::<u64>();
            let value = rng.gen::<u64>();
            key_to_value.insert(key, value);
            map.insert(&key, &value);
        }
        for _ in 0..100 {
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
