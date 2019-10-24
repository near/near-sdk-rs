//! A set implemented on a trie. Unlike `std::collections::HashSet` the elements in this set are not
//! hashed but are instead serialized.
use crate::collections::next_trie_id;
use crate::env;
use borsh::{BorshDeserialize, BorshSerialize};
use near_vm_logic::types::IteratorIndex;
use std::marker::PhantomData;

#[derive(BorshSerialize, BorshDeserialize)]
pub struct Set<T> {
    len: u64,
    prefix: Vec<u8>,
    #[borsh_skip]
    element: PhantomData<T>,
}

impl<T> Set<T> {
    /// Returns the number of elements in the set, also referred to as its 'size'.
    pub fn len(&self) -> u64 {
        self.len
    }
}

impl<T> Default for Set<T> {
    fn default() -> Self {
        Self::new(next_trie_id())
    }
}

impl<T> Set<T> {
    /// Create new set with zero elements. Use `id` as a unique identifier.
    pub fn new(id: Vec<u8>) -> Self {
        Self { len: 0, prefix: id, element: PhantomData }
    }
}

impl<T> Set<T>
where
    T: BorshSerialize + BorshDeserialize,
{
    /// Serializes element into an array of bytes.
    fn serialize_element(&self, element: &T) -> Vec<u8> {
        let mut res = self.prefix.clone();
        let data = element.try_to_vec().expect("Element should be serializable with Borsh.");
        res.extend(data);
        res
    }

    /// Deserializes element, taking prefix into account.
    fn deserialize_element(prefix: &[u8], raw_element: &[u8]) -> T {
        let element = &raw_element[prefix.len()..];
        T::try_from_slice(element).expect("Element should be deserializable with Borsh.")
    }

    /// An iterator visiting all elements. The iterator element type is `T`.
    pub fn iter<'a>(&'a self) -> impl Iterator<Item = T> + 'a {
        let prefix = self.prefix.clone();
        self.raw_elements().map(move |k| Self::deserialize_element(&prefix, &k))
    }

    /// Returns `true` if the set contains a value
    pub fn contains(&self, element: &T) -> bool {
        let raw_element = self.serialize_element(element);
        env::storage_read(&raw_element).is_some()
    }

    /// Removes an element from the set, returning `true` if the element was present.
    pub fn remove(&mut self, element: &T) -> bool {
        let raw_element = self.serialize_element(element);
        if env::storage_remove(&raw_element) {
            self.len -= 1;
            true
        } else {
            false
        }
    }

    /// Inserts an element into the set. If element was already present returns `true`.
    pub fn insert(&mut self, element: &T) -> bool {
        let raw_element = self.serialize_element(element);
        if env::storage_write(&raw_element, &[]) {
            true
        } else {
            self.len += 1;
            false
        }
    }

    /// Copies elements into an `std::vec::Vec`.
    pub fn to_vec(&self) -> std::vec::Vec<T> {
        self.iter().collect()
    }

    /// Raw serialized elements.
    fn raw_elements(&self) -> IntoSetRawElements {
        let iterator_id = env::storage_iter_prefix(&self.prefix);
        IntoSetRawElements { iterator_id }
    }
    /// Clears the set, removing all elements.
    pub fn clear(&mut self) {
        let elements: Vec<Vec<u8>> = self.raw_elements().collect();
        for element in elements {
            env::storage_remove(&element);
        }
        self.len = 0;
    }

    pub fn extend<IT: IntoIterator<Item = T>>(&mut self, iter: IT) {
        for el in iter {
            let element = self.serialize_element(&el);
            if !env::storage_write(&element, &[]) {
                self.len += 1;
            }
        }
    }
}

/// Non-consuming iterator over raw serialized elements of `Set<T>`.
pub struct IntoSetRawElements {
    iterator_id: IteratorIndex,
}

impl Iterator for IntoSetRawElements {
    type Item = Vec<u8>;

    fn next(&mut self) -> Option<Self::Item> {
        if env::storage_iter_next(self.iterator_id) {
            env::storage_iter_key_read()
        } else {
            None
        }
    }
}
