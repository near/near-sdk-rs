//! A map implemented on a trie. Unlike `std::collections::HashMap` the keys in this map are not
//! hashed but are instead serialized.
use crate::collections::next_trie_id;
use crate::Environment;
use borsh::{BorshDeserialize, BorshSerialize};
use near_vm_logic::types::IteratorIndex;
use std::marker::PhantomData;

#[derive(BorshSerialize, BorshDeserialize)]
pub struct Map<K, V> {
    len: u64,
    prefix: Vec<u8>,
    #[borsh_skip]
    key: PhantomData<K>,
    #[borsh_skip]
    value: PhantomData<V>,
}

impl<K, V> Map<K, V> {
    /// Returns the number of elements in the map, also referred to as its 'size'.
    pub fn len(&self) -> u64 {
        self.len
    }
}

impl<K, V> Default for Map<K, V> {
    fn default() -> Self {
        Self::new(next_trie_id())
    }
}

impl<K, V> Map<K, V> {
    /// Create new map with zero elements. Use `id` as a unique identifier.
    pub fn new(id: Vec<u8>) -> Self {
        Self { len: 0, prefix: id, key: PhantomData, value: PhantomData }
    }
}

impl<K, V> Map<K, V>
where
    K: BorshSerialize + BorshDeserialize,
    V: BorshSerialize + BorshDeserialize,
{
    /// Serializes key into an array of bytes.
    fn serialize_key(&self, key: K) -> Vec<u8> {
        let mut res = self.prefix.clone();
        let data = key.try_to_vec().expect("Key should be serializable with Borsh.");
        res.extend(data);
        res
    }

    /// Serializes value into an array of bytes.
    fn serialize_value(&self, value: V) -> Vec<u8> {
        value.try_to_vec().expect("Value should be serializable with Borsh.")
    }

    /// Deserializes key, taking prefix into account.
    fn deserialize_key(prefix: &[u8], raw_key: &[u8]) -> K {
        let key = &raw_key[prefix.len()..];
        K::try_from_slice(key).expect("Key should be deserializable with Borsh.")
    }

    /// Deserializes value.
    fn deserialize_value(value: &[u8]) -> V {
        V::try_from_slice(value).expect("Value should be deserializable with Borsh.")
    }

    /// An iterator visiting all keys. The iterator element type is `K`.
    pub fn keys<'a>(&'a self, env: &'a mut Environment) -> impl Iterator<Item = K> + 'a {
        let prefix = self.prefix.clone();
        self.raw_keys(env).map(move |k| Self::deserialize_key(&prefix, &k))
    }

    /// An iterator visiting all values. The iterator element type is `V`.
    pub fn values<'a>(&'a self, env: &'a mut Environment) -> impl Iterator<Item = V> + 'a {
        self.raw_values(env).map(|v| Self::deserialize_value(&v))
    }

    /// Removes a key from the map, returning the value at the key if the key was previously in the map.
    pub fn remove(&mut self, env: &mut Environment, key: K) -> Option<V> {
        let raw_key = self.serialize_key(key);
        if env.storage_remove(&raw_key) {
            self.len -= 1;
            let data = env
                .storage_get_evicted()
                .expect("The removal signaled that the value was evicted.");
            Some(Self::deserialize_value(&data))
        } else {
            None
        }
    }

    /// Inserts a key-value pair into the map.
    ///
    /// If the map did not have this key present, [`None`] is returned.
    ///
    /// If the map did have this key present, the value is updated, and the old
    /// value is returned.
    pub fn insert(&mut self, env: &mut Environment, key: K, value: V) -> Option<V> {
        let key = self.serialize_key(key);
        let value = self.serialize_value(value);
        if env.storage_write(&key, &value) {
            let data =
                env.storage_get_evicted().expect("The insert signaled that the value was evicted.");
            Some(Self::deserialize_value(&data))
        } else {
            self.len += 1;
            None
        }
    }

    /// Copies elements into an `std::vec::Vec`.
    pub fn to_vec(&self, env: &mut Environment) -> std::vec::Vec<(K, V)> {
        self.iter(env).collect()
    }

    /// Raw serialized keys.
    fn raw_keys<'a>(&'a self, env: &'a mut Environment) -> IntoMapRawKeys<'a> {
        let iterator_id = env.storage_iter_prefix(&self.prefix);
        IntoMapRawKeys { iterator_id, env }
    }

    /// Raw serialized values.
    fn raw_values<'a>(&'a self, env: &'a mut Environment) -> IntoMapRawValues<'a> {
        let iterator_id = env.storage_iter_prefix(&self.prefix);
        IntoMapRawValues { iterator_id, env }
    }

    /// Clears the map, removing all elements.
    pub fn clear(&mut self, env: &mut Environment) {
        let keys: Vec<Vec<u8>> = self.raw_keys(env).collect();
        for key in keys {
            env.storage_remove(&key);
        }
        self.len = 0;
    }

    pub fn iter<'a>(&'a self, env: &'a mut Environment) -> IntoMapRef<'a, K, V> {
        let iterator_id = env.storage_iter_prefix(&self.prefix);
        IntoMapRef { iterator_id, map: self, env }
    }

    pub fn extend<IT: IntoIterator<Item = (K, V)>>(&mut self, env: &mut Environment, iter: IT) {
        for (el_key, el_value) in iter {
            let key = self.serialize_key(el_key);
            let value = self.serialize_value(el_value);
            if !env.storage_write(&key, &value) {
                self.len += 1;
            }
        }
    }
}

/// Non-consuming iterator for `Map<K, V>`.
pub struct IntoMapRef<'a, K, V> {
    iterator_id: IteratorIndex,
    map: &'a Map<K, V>,
    env: &'a mut Environment,
}

impl<'a, K, V> Iterator for IntoMapRef<'a, K, V>
where
    K: BorshSerialize + BorshDeserialize,
    V: BorshSerialize + BorshDeserialize,
{
    type Item = (K, V);

    fn next(&mut self) -> Option<Self::Item> {
        if self.env.storage_iter_next(self.iterator_id) {
            let key = self.env.storage_iter_key_read()?;
            let value = self.env.storage_iter_value_read()?;
            Some((
                Map::<K, V>::deserialize_key(&self.map.prefix, &key),
                Map::<K, V>::deserialize_value(&value),
            ))
        } else {
            None
        }
    }
}

/// Non-consuming iterator over raw serialized keys of `Map<K, V>`.
pub struct IntoMapRawKeys<'a> {
    iterator_id: IteratorIndex,
    env: &'a mut Environment,
}

impl<'a> Iterator for IntoMapRawKeys<'a> {
    type Item = Vec<u8>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.env.storage_iter_next(self.iterator_id) {
            self.env.storage_iter_key_read()
        } else {
            None
        }
    }
}

/// Non-consuming iterator over serialized values of `Map<K, V>`.
pub struct IntoMapRawValues<'a> {
    iterator_id: u64,
    env: &'a mut Environment,
}

impl<'a> Iterator for IntoMapRawValues<'a> {
    type Item = Vec<u8>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.env.storage_iter_next(self.iterator_id) {
            self.env.storage_iter_value_read()
        } else {
            None
        }
    }
}
