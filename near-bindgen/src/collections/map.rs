//! A map implemented on a trie. Unlike `std::collections::HashMap` the keys in this map are not
//! hashed but are instead serialized.
use crate::collections::next_trie_id;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::marker::PhantomData;

/// Empty value. Set is implemented through the trie and does not store anything in the values.
static EMPTY: [u8; 0] = [];

#[derive(Serialize, Deserialize)]
pub struct Map<K, V> {
    len: usize,
    id: String,
    key: PhantomData<K>,
    value: PhantomData<V>,
}

impl<K, V> Map<K, V> {
    /// Head is the key that precedes all keys of real elements. This is used for efficient
    /// iteration over the elements of map.
    pub(crate) fn head(&self) -> Vec<u8> {
        format!("{}Key0", self.id).into_bytes()
    }

    /// Tail is the key that follows all keys of real elements. This is used for efficient
    /// iteration over the elements of map.
    pub(crate) fn tail(&self) -> Vec<u8> {
        format!("{}Key2", self.id).into_bytes()
    }

    /// Get the prefix of the keys.
    fn key_prefix(&self) -> Vec<u8> {
        format!("{}Key1", self.id).into_bytes()
    }

    /// Returns the number of elements in the map, also referred to as its 'size'.
    pub fn len(&self) -> usize {
        self.len
    }

    fn set_len(&mut self, value: usize) {
        self.len = value;
    }
}

impl<K, V> Default for Map<K, V>
where
    K: Serialize + DeserializeOwned,
    V: Serialize + DeserializeOwned,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<K, V> Map<K, V>
where
    K: Serialize + DeserializeOwned,
    V: Serialize + DeserializeOwned,
{
    /// Serializes key into an array of bytes.
    fn serialize_key(&self, key: K) -> Vec<u8> {
        let mut res = self.key_prefix();
        let data = bincode::serialize(&key).unwrap();
        res.extend(data);
        res
    }

    /// Serializes key into an array of bytes.
    fn serialize_value(&self, value: V) -> Vec<u8> {
        bincode::serialize(&value).unwrap()
    }

    /// Deserializes key, taking prefix into account.
    fn deserialize_key(&self, key: &[u8]) -> K {
        let key = &key[self.key_prefix().len()..];
        bincode::deserialize(&key).unwrap()
    }

    /// Deserializes value.
    fn deserialize_value(&self, value: &[u8]) -> V {
        bincode::deserialize(&value).unwrap()
    }
    /// Create new map with zero elements.
    pub fn new() -> Self {
        let res = Self { len: 0, id: next_trie_id(), key: PhantomData, value: PhantomData };
        // Add the marker records.
        let head = res.head();
        let tail = res.tail();
        crate::CONTEXT.storage_write(&head, &EMPTY);
        crate::CONTEXT.storage_write(&tail, &EMPTY);
        res
    }

    /// An iterator visiting all keys. The iterator element type is `K`.
    pub fn keys<'a>(&'a self) -> impl Iterator<Item = K> + 'a {
        let key_prefix_len = self.key_prefix().len();
        self.raw_keys().map(move |k| {
            let key = &k[key_prefix_len..];
            bincode::deserialize(&key).unwrap()
        })
    }

    /// An iterator visiting all values. The iterator element type is `V`.
    pub fn values<'a>(&'a self) -> impl Iterator<Item = V> + 'a {
        self.into_iter().map(|(_, v)| v)
    }

    /// Removes a key from the map, returning the value at the key if the key was previously in the map.
    pub fn remove(&mut self, key: K) -> Option<V> {
        let key = self.serialize_key(key);
        if !crate::CONTEXT.storage_has_key(&key) {
            return None;
        }
        let data = crate::CONTEXT.storage_read(&key);
        let result = bincode::deserialize(&data).ok().unwrap();
        crate::CONTEXT.storage_remove(&key);
        self.set_len(self.len() - 1);
        Some(result)
    }

    /// Inserts a key-value pair into the map.
    ///
    /// If the map did not have this key present, [`None`] is returned.
    ///
    /// If the map did have this key present, the value is updated, and the old
    /// value is returned.
    pub fn insert(&mut self, key: K, value: V) -> Option<V> {
        let key = self.serialize_key(key);
        let res = if crate::CONTEXT.storage_has_key(&key) {
            let value = crate::CONTEXT.storage_read(&key);
            Some(self.deserialize_value(&value))
        } else {
            self.set_len(self.len() + 1);
            None
        };

        let value = self.serialize_value(value);
        crate::CONTEXT.storage_write(&key, &value);
        res
    }

    /// Copies elements into an `std::vec::Vec`.
    pub fn to_vec(&self) -> std::vec::Vec<(K, V)> {
        let res = self.into_iter().collect();
        res
    }

    /// Raw serialized keys.
    fn raw_keys(&self) -> IntoMapRawKeys<K, V> {
        let start = self.head();
        let end = self.tail();
        let iterator_id = crate::CONTEXT.storage_range(&start, &end);
        IntoMapRawKeys { iterator_id, map: self, ended: false }
    }

    /// Clears the map, removing all elements.
    pub fn clear(&mut self) {
        let keys: Vec<Vec<u8>> = self.raw_keys().collect();
        for key in keys {
            crate::CONTEXT.storage_remove(&key);
        }
        self.set_len(0);
    }
}

impl<'a, K, V> IntoIterator for &'a Map<K, V>
where
    K: Serialize + DeserializeOwned,
    V: Serialize + DeserializeOwned,
{
    type Item = (K, V);
    type IntoIter = IntoMapRef<'a, K, V>;

    fn into_iter(self) -> Self::IntoIter {
        if self.len() == 0 {
            return IntoMapRef { iterator_id: 0, map: self, ended: true };
        }
        let start = self.head();
        let end = self.tail();
        let iterator_id = crate::CONTEXT.storage_range(&start, &end);
        IntoMapRef { iterator_id, map: self, ended: false }
    }
}

impl<'a, K, V> IntoIterator for &'a mut Map<K, V>
where
    K: Serialize + DeserializeOwned,
    V: Serialize + DeserializeOwned,
{
    type Item = (K, V);
    type IntoIter = IntoMapRef<'a, K, V>;

    fn into_iter(self) -> Self::IntoIter {
        if self.len() == 0 {
            return IntoMapRef { iterator_id: 0, map: self, ended: true };
        }
        let start = self.head();
        let end = self.tail();
        let iterator_id = crate::CONTEXT.storage_range(&start, &end);
        IntoMapRef { iterator_id, map: self, ended: false }
    }
}

/// Non-consuming iterator for `Map<K, V>`.
pub struct IntoMapRef<'a, K, V> {
    iterator_id: u32,
    #[allow(dead_code)]
    map: &'a Map<K, V>,
    ended: bool,
}

impl<'a, K, V> Iterator for IntoMapRef<'a, K, V>
where
    K: Serialize + DeserializeOwned,
    V: Serialize + DeserializeOwned,
{
    type Item = (K, V);

    fn next(&mut self) -> Option<Self::Item> {
        if self.ended {
            return None;
        }
        let mut key_data = crate::CONTEXT.storage_peek(self.iterator_id);
        if key_data == self.map.head() {
            crate::CONTEXT.storage_iter_next(self.iterator_id);
            key_data = crate::CONTEXT.storage_peek(self.iterator_id);
        }
        if key_data.is_empty() || key_data == self.map.tail() {
            return None;
        }
        let value_data = crate::CONTEXT.storage_read(&key_data);
        let ended = !crate::CONTEXT.storage_iter_next(self.iterator_id);
        if ended {
            self.ended = true;
        }
        Some((self.map.deserialize_key(&key_data), self.map.deserialize_value(&value_data)))
    }
}

/// Non-consuming iterator over raw serialized keys of `Map<K, V>`.
pub struct IntoMapRawKeys<'a, K, V> {
    iterator_id: u32,
    #[allow(dead_code)]
    map: &'a Map<K, V>,
    ended: bool,
}

impl<'a, K, V> Iterator for IntoMapRawKeys<'a, K, V>
where
    K: Serialize + DeserializeOwned,
    V: Serialize + DeserializeOwned,
{
    type Item = Vec<u8>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.ended {
            return None;
        }
        let mut key_data = crate::CONTEXT.storage_peek(self.iterator_id);
        if key_data == self.map.head() {
            crate::CONTEXT.storage_iter_next(self.iterator_id);
            key_data = crate::CONTEXT.storage_peek(self.iterator_id);
        }
        if key_data.is_empty() || key_data == self.map.tail() {
            return None;
        }
        let ended = !crate::CONTEXT.storage_iter_next(self.iterator_id);
        if ended {
            self.ended = true;
        }
        Some(key_data)
    }
}

impl<K, V> Extend<(K, V)> for Map<K, V>
where
    K: Serialize + DeserializeOwned,
    V: Serialize + DeserializeOwned,
{
    fn extend<I: IntoIterator<Item = (K, V)>>(&mut self, iter: I) {
        let mut len = self.len();
        for (el_key, el_value) in iter {
            let key = self.serialize_key(el_key);
            let value = self.serialize_value(el_value);
            crate::CONTEXT.storage_write(&key, &value);
            len += 1;
        }
        self.set_len(len);
    }
}
