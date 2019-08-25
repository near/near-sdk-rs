//! A set implemented on a trie. Unlike `std::collections::HashSet` the elements in this set are not
//! hashed but are instead serialized.
use crate::collections::next_trie_id;
use crate::Environment;
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
    fn serialize_element(&self, element: T) -> Vec<u8> {
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
    pub fn iter<'a>(&'a self, env: &'a mut Environment) -> impl Iterator<Item = T> + 'a {
        let prefix = self.prefix.clone();
        self.raw_elements(env).map(move |k| Self::deserialize_element(&prefix, &k))
    }

    /// Removes an element from the set, returning `true` if the element was present.
    pub fn remove(&mut self, env: &mut Environment, element: T) -> bool {
        let raw_element = self.serialize_element(element);
        if env.storage_remove(&raw_element) {
            self.len -= 1;
            true
        } else {
            false
        }
    }

    /// Inserts an element into the set. If element was already present returns `true`.
    pub fn insert(&mut self, env: &mut Environment, element: T) -> bool {
        let raw_element = self.serialize_element(element);
        if env.storage_write(&raw_element, &[]) {
            true
        } else {
            self.len += 1;
            false
        }
    }

    /// Copies elements into an `std::vec::Vec`.
    pub fn to_vec(&self, env: &mut Environment) -> std::vec::Vec<T> {
        self.iter(env).collect()
    }

    /// Raw serialized elements.
    fn raw_elements<'a>(&'a self, env: &'a mut Environment) -> IntoSetRawElements<'a> {
        let iterator_id = env.storage_iter_prefix(&self.prefix);
        IntoSetRawElements { iterator_id, env }
    }
    /// Clears the set, removing all elements.
    pub fn clear(&mut self, env: &mut Environment) {
        let elements: Vec<Vec<u8>> = self.raw_elements(env).collect();
        for element in elements {
            env.storage_remove(&element);
        }
        self.len = 0;
    }

    pub fn extend<IT: IntoIterator<Item = T>>(&mut self, env: &mut Environment, iter: IT) {
        for el in iter {
            let element = self.serialize_element(el);
            if !env.storage_write(&element, &[]) {
                self.len += 1;
            }
        }
    }
}

/// Non-consuming iterator over raw serialized elements of `Set<T>`.
pub struct IntoSetRawElements<'a> {
    iterator_id: IteratorIndex,
    env: &'a mut Environment,
}

impl<'a> Iterator for IntoSetRawElements<'a> {
    type Item = Vec<u8>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.env.storage_iter_next(self.iterator_id) {
            self.env.storage_iter_key_read()
        } else {
            None
        }
    }
}
