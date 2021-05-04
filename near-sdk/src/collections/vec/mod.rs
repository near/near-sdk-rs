//! A vector implemented on a trie. Unlike standard vector does not support insertion and removal
//! of an element results in the last element being placed in the empty position.

use crate::collections::append_slice;
use crate::{env, IntoStorageKey};
use borsh::{BorshDeserialize, BorshSerialize};
use std::{cell::RefCell, collections::BTreeMap, marker::PhantomData};

const ERR_INCONSISTENT_STATE: &[u8] = b"The collection is an inconsistent state. Did previous smart contract execution terminate unexpectedly?";
const ERR_ELEMENT_DESERIALIZATION: &[u8] = b"Cannot deserialize element";
const ERR_ELEMENT_SERIALIZATION: &[u8] = b"Cannot serialize element";
const ERR_INDEX_OUT_OF_BOUNDS: &[u8] = b"Index out of bounds";

fn expect_consistent_state<T>(val: Option<T>) -> T {
    val.unwrap_or_else(|| env::panic(ERR_INCONSISTENT_STATE))
}

/// An iterable implementation of vector that stores its content on the trie.
/// Uses the following map: index -> element.
#[derive(BorshSerialize, BorshDeserialize)]
#[cfg_attr(not(feature = "expensive-debug"), derive(Debug))]
pub struct Vector<T> {
    // TODO: determine why u64 was used previously -- is it required? u32 faster in wasm env
    len: u32,
    prefix: Vec<u8>,
    #[borsh_skip]
    cache: RefCell<BTreeMap<u32, T>>,
}

impl<T> Vector<T> {
    /// Returns the number of elements in the vector, also referred to as its size.
    pub fn len(&self) -> u32 {
        self.len
    }

    /// Returns `true` if the vector contains no elements.
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Create new vector with zero elements. Use `id` as a unique identifier on the trie.
    pub fn new<S>(prefix: S) -> Self
    where
        S: IntoStorageKey,
    {
        Self { len: 0, prefix: prefix.into_storage_key(), cache: Default::default() }
    }

    fn index_to_lookup_key(&self, index: u32) -> Vec<u8> {
        append_slice(&self.prefix, &index.to_le_bytes()[..])
    }

    /// Returns the serialized element by index or `None` if it is not present.
    fn get_raw(&self, index: u32) -> Option<Vec<u8>> {
        if index >= self.len {
            return None;
        }
        let lookup_key = self.index_to_lookup_key(index);
        Some(expect_consistent_state(env::storage_read(&lookup_key)))
    }

    /// Removes an element from the vector and returns it in serialized form.
    /// The removed element is replaced by the last element of the vector.
    /// Does not preserve ordering, but is `O(1)`.
    ///
    /// # Panics
    ///
    /// Panics if `index` is out of bounds.
    fn swap_remove_raw(&mut self, index: u32) -> Vec<u8> {
        if index >= self.len {
            env::panic(ERR_INDEX_OUT_OF_BOUNDS)
        } else if index + 1 == self.len {
            expect_consistent_state(self.pop_raw())
        } else {
            let lookup_key = self.index_to_lookup_key(index);
            let raw_last_value = self.pop_raw().expect("checked `index < len` above, so `len > 0`");
            if env::storage_write(&lookup_key, &raw_last_value) {
                expect_consistent_state(env::storage_get_evicted())
            } else {
                env::panic(ERR_INCONSISTENT_STATE)
            }
        }
    }

    /// Appends a serialized element to the back of the collection.
    fn push_raw(&mut self, raw_element: &[u8]) {
        let lookup_key = self.index_to_lookup_key(self.len);
        self.len += 1;
        env::storage_write(&lookup_key, raw_element);
    }

    /// Removes the last element from a vector and returns it without deserializing, or `None` if it is empty.
    fn pop_raw(&mut self) -> Option<Vec<u8>> {
        if self.is_empty() {
            None
        } else {
            let last_index = self.len - 1;
            let last_lookup_key = self.index_to_lookup_key(last_index);

            self.len -= 1;
            let raw_last_value = if env::storage_remove(&last_lookup_key) {
                expect_consistent_state(env::storage_get_evicted())
            } else {
                env::panic(ERR_INCONSISTENT_STATE)
            };
            Some(raw_last_value)
        }
    }

    /// Inserts a serialized element at `index`, returns a serialized evicted element.
    ///
    /// # Panics
    ///
    /// If `index` is out of bounds.
    fn replace_raw(&mut self, index: u32, raw_element: &[u8]) -> Vec<u8> {
        if index >= self.len {
            env::panic(ERR_INDEX_OUT_OF_BOUNDS)
        } else {
            let lookup_key = self.index_to_lookup_key(index);
            if env::storage_write(&lookup_key, &raw_element) {
                expect_consistent_state(env::storage_get_evicted())
            } else {
                env::panic(ERR_INCONSISTENT_STATE);
            }
        }
    }

    /// Iterate over raw serialized elements.
    fn iter_raw(&self) -> impl Iterator<Item = Vec<u8>> + '_ {
        (0..self.len).map(move |i| {
            let lookup_key = self.index_to_lookup_key(i);
            expect_consistent_state(env::storage_read(&lookup_key))
        })
    }

    /// Extends vector from the given collection of serialized elements.
    fn extend_raw<IT: IntoIterator<Item = Vec<u8>>>(&mut self, iter: IT) {
        for el in iter {
            self.push_raw(&el)
        }
    }

    /// Removes all elements from the collection.
    pub fn clear(&mut self) {
        for i in 0..self.len {
            let lookup_key = self.index_to_lookup_key(i);
            env::storage_remove(&lookup_key);
        }
        self.len = 0;
        *self.cache.borrow_mut() = Default::default();
    }
}

impl<T> Vector<T>
where
    T: BorshSerialize,
{
    fn serialize_element(element: &T) -> Vec<u8> {
        element.try_to_vec().unwrap_or_else(|_| env::panic(ERR_ELEMENT_SERIALIZATION))
    }

    /// Appends an element to the back of the collection.
    pub fn push(&mut self, element: T) {
        let raw_element = Self::serialize_element(&element);
        let idx = self.len;
        self.push_raw(&raw_element);
        self.cache.borrow_mut().insert(idx, element);
    }

    /// Extends vector from the given collection.
    pub fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
        for el in iter {
            self.push(el)
        }
    }
}
