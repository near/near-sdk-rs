//! A set implemented on a trie. Unlike `std::collections::HashSet` the keys in this set are not
//! hashed but are instead serialized.
use crate::collections::next_trie_id;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::marker::PhantomData;

/// Empty value. Set is implemented through the trie and does not store anything in the values.
static EMPTY: [u8; 0] = [];

#[derive(Serialize, Deserialize)]
pub struct Set<T> {
    len: usize,
    id: String,
    element: PhantomData<T>,
}

impl<T> Set<T> {
    /// Head is the element that precedes all real elements. This is used for efficient iteration
    /// over the elements of set.
    pub(crate) fn head(&self) -> Vec<u8> {
        format!("{}Element0", self.id).into_bytes()
    }

    /// Tail is the element that follows all real elements. This is used for efficient iteration
    /// over the elements of set.
    pub(crate) fn tail(&self) -> Vec<u8> {
        format!("{}Element2", self.id).into_bytes()
    }

    /// Get the prefix of the elements.
    fn prefix(&self) -> Vec<u8> {
        format!("{}Element1", self.id).into_bytes()
    }

    /// Returns the number of elements in the set, also referred to as its 'size'.
    pub fn len(&self) -> usize {
        self.len
    }

    fn set_len(&mut self, value: usize) {
        self.len = value;
    }
}

impl<T> Default for Set<T>
where
    T: Serialize + DeserializeOwned,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<T> Set<T>
where
    T: Serialize + DeserializeOwned,
{
    /// Serializes element into an array of bytes.
    fn serialize_element(&self, element: T) -> Vec<u8> {
        let mut res = self.prefix();
        let data = bincode::serialize(&element).unwrap();
        res.extend(data);
        res
    }

    /// Deserializes element, taking prefix into account.
    fn deserialize_element(&self, element: &[u8]) -> T {
        let element = &element[self.prefix().len()..];
        bincode::deserialize(&element).unwrap()
    }

    /// Create new set with zero elements.
    pub fn new() -> Self {
        let res = Self { len: 0, id: next_trie_id(), element: PhantomData };
        // Add the marker records.
        let head = res.head();
        let tail = res.tail();
        crate::CONTEXT.storage_write(&head, &EMPTY);
        crate::CONTEXT.storage_write(&tail, &EMPTY);
        res
    }

    /// Removes a value from the set. Returns whether the value was present in the set.
    pub fn remove(&mut self, value: T) -> bool {
        let key = self.serialize_element(value);
        if !crate::CONTEXT.storage_has_key(&key) {
            return false;
        }
        crate::CONTEXT.storage_remove(&key);
        self.set_len(self.len() - 1);
        true
    }

    /// If the set did have this key present, the value is updated, and the old
    /// value is returned.
    /// Adds a value to the set.
    ///
    /// If the set did not have this value present, true is returned.
    ///
    /// If the set did have this value present, false is returned.
    pub fn insert(&mut self, value: T) -> bool {
        let key = self.serialize_element(value);
        if crate::CONTEXT.storage_has_key(&key) {
            crate::CONTEXT.storage_write(&key, &EMPTY);
            false
        } else {
            self.set_len(self.len() + 1);
            true
        }
    }

    /// Copies elements into an `std::vec::Vec`.
    pub fn to_vec(&self) -> std::vec::Vec<T> {
        let res = self.into_iter().collect();
        res
    }

    /// Raw serialized keys.
    fn raw_keys(&self) -> IntoSetRawKeys<T> {
        let start = self.head();
        let end = self.tail();
        let iterator_id = crate::CONTEXT.storage_range(&start, &end);
        IntoSetRawKeys { iterator_id, set: self, ended: false }
    }

    /// Clears the set, removing all elements.
    pub fn clear(&mut self) {
        let keys: Vec<Vec<u8>> = self.raw_keys().collect();
        for key in keys {
            crate::CONTEXT.storage_remove(&key);
        }
        self.set_len(0);
    }
}

impl<'a, T> IntoIterator for &'a Set<T>
where
    T: Serialize + DeserializeOwned,
{
    type Item = T;
    type IntoIter = IntoSetRef<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        if self.len() == 0 {
            return IntoSetRef { iterator_id: 0, set: self, ended: true };
        }
        let start = self.head();
        let end = self.tail();
        let iterator_id = crate::CONTEXT.storage_range(&start, &end);
        IntoSetRef { iterator_id, set: self, ended: false }
    }
}

impl<'a, T> IntoIterator for &'a mut Set<T>
where
    T: Serialize + DeserializeOwned,
{
    type Item = T;
    type IntoIter = IntoSetRef<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        if self.len() == 0 {
            return IntoSetRef { iterator_id: 0, set: self, ended: true };
        }
        let start = self.head();
        let end = self.tail();
        let iterator_id = crate::CONTEXT.storage_range(&start, &end);
        IntoSetRef { iterator_id, set: self, ended: false }
    }
}

/// Non-consuming iterator for `Set<T>`.
pub struct IntoSetRef<'a, T> {
    iterator_id: u32,
    #[allow(dead_code)]
    set: &'a Set<T>,
    ended: bool,
}

impl<'a, T> Iterator for IntoSetRef<'a, T>
where
    T: Serialize + DeserializeOwned,
{
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.ended {
            return None;
        }
        let mut key_data = crate::CONTEXT.storage_peek(self.iterator_id);
        if key_data == self.set.head() {
            crate::CONTEXT.storage_iter_next(self.iterator_id);
            key_data = crate::CONTEXT.storage_peek(self.iterator_id);
        }
        if key_data.is_empty() || key_data == self.set.tail() {
            return None;
        }
        let ended = !crate::CONTEXT.storage_iter_next(self.iterator_id);
        if ended {
            self.ended = true;
        }
        Some(self.set.deserialize_element(&key_data))
    }
}

/// Non-consuming iterator over raw serialized elements of `Set<T>`.
pub struct IntoSetRawKeys<'a, T> {
    iterator_id: u32,
    #[allow(dead_code)]
    set: &'a Set<T>,
    ended: bool,
}

impl<'a, T> Iterator for IntoSetRawKeys<'a, T>
where
    T: Serialize + DeserializeOwned,
{
    type Item = Vec<u8>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.ended {
            return None;
        }
        let mut key_data = crate::CONTEXT.storage_peek(self.iterator_id);
        if key_data == self.set.head() {
            crate::CONTEXT.storage_iter_next(self.iterator_id);
            key_data = crate::CONTEXT.storage_peek(self.iterator_id);
        }
        if key_data.is_empty() || key_data == self.set.tail() {
            return None;
        }
        let ended = !crate::CONTEXT.storage_iter_next(self.iterator_id);
        if ended {
            self.ended = true;
        }
        Some(key_data)
    }
}

impl<T> Extend<T> for Set<T>
where
    T: Serialize + DeserializeOwned,
{
    fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
        let mut len = self.len();
        for el in iter {
            let key = self.serialize_element(el);
            crate::CONTEXT.storage_write(&key, &EMPTY);
            len += 1;
        }
        self.set_len(len);
    }
}
