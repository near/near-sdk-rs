//! A vector implemented on a trie. Unlike standard vector does not support insertion and removal
//! of an element results in the last element being placed in the empty position.

mod impls;
mod iter;

use crate::collections::append_slice;
use crate::{env, CacheCell, CacheEntry, EntryState, IntoStorageKey};
use borsh::{BorshDeserialize, BorshSerialize};
use std::collections::{btree_map::Entry, BTreeMap};
use std::ptr::NonNull;

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
    /// Cache for loads and intermediate changes to the underlying vector.
    /// The cached entries are wrapped in a [`Box`] to avoid existing pointers from being
    /// invalidated.
    cache: CacheCell<BTreeMap<u32, Box<CacheEntry<T>>>>,
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
        *self.cache.as_inner_mut() = Default::default();
    }

    /// Inserts the current value into cache, does not load value into cache from storage if
    /// the cache entry if not filled.
    fn insert(&mut self, index: u32, value: Option<T>) {
        match self.cache.as_inner_mut().entry(index) {
            Entry::Occupied(mut occupied) => {
                occupied.get_mut().replace(value);
            }
            Entry::Vacant(vacant) => {
                vacant.insert(Box::new(CacheEntry::new_modified(value)));
            }
        }
    }

    /// Loads value from storage into cache, if it does not already exist.
    /// This function must be unsafe because it requires modifying the cache with an immutable
    /// reference.
    unsafe fn load(&self, index: u32) -> NonNull<CacheEntry<T>> {
        // match self.cache.as_inner_mut().entry(index) {
        //     Entry::Occupied(mut occupied) => {
        //         occupied.get_mut().replace(value);
        //     },
        //     Entry::Vacant(vacant) => {
        //         vacant.insert(Box::new(CacheEntry::new_modified(value)));
        //     }
        // }
        todo!()
    }

    /// Loads value from storage into cache, and returns a mutable reference to the loaded value.
    /// This function is safe because a mutable reference of self is used.
    fn load_mut(&mut self, index: u32) -> &mut CacheEntry<T> {
        todo!()
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

        // Element guaranteed to not exist in cache, can insert without loading entry
        self.cache.as_inner_mut().insert(idx, Box::new(CacheEntry::new_modified(Some(element))));
    }

    // TODO move this to extend trait
    /// Extends vector from the given collection.
    pub fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
        for el in iter {
            self.push(el)
        }
    }
}

impl<T> Vector<T>
where
    T: BorshDeserialize,
{
    fn deserialize_element(raw_element: &[u8]) -> T {
        T::try_from_slice(&raw_element).unwrap_or_else(|_| env::panic(ERR_ELEMENT_DESERIALIZATION))
    }

    /// Returns the element by index or `None` if it is not present.
    pub fn get(&self, index: u32) -> Option<&T> {
        // TODO doc safety
        unsafe { &*self.load(index).as_ptr() }.value().as_ref()
    }

    /// Removes an element from the vector and returns it.
    /// The removed element is replaced by the last element of the vector.
    /// Does not preserve ordering, but is `O(1)`.
    ///
    /// # Panics
    ///
    /// Panics if `index` is out of bounds.
    pub fn swap_remove(&mut self, index: u32) -> T {
        let raw_evicted = self.swap_remove_raw(index);
        Self::deserialize_element(&raw_evicted)
    }

    /// Removes the last element from a vector and returns it, or `None` if it is empty.
    pub fn pop(&mut self) -> Option<T> {
        if self.is_empty() {
            None
        } else {
            let last_idx = self.len - 1;
            self.len = last_idx;

            // Replace current value with none, and return the existing value
            self.load_mut(last_idx).replace(None)
        }
    }
}
