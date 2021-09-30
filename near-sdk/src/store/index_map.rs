use std::fmt;

use borsh::{BorshDeserialize, BorshSerialize};
use once_cell::unsync::OnceCell;

use crate::utils::StableMap;
use crate::{env, CacheEntry, EntryState, IntoStorageKey};

const ERR_ELEMENT_DESERIALIZATION: &str = "Cannot deserialize element";
const ERR_ELEMENT_SERIALIZATION: &str = "Cannot serialize element";

#[derive(BorshSerialize, BorshDeserialize)]
pub(crate) struct IndexMap<T>
where
    T: BorshSerialize,
{
    pub(crate) prefix: Box<[u8]>,
    /// Cache for loads and intermediate changes to the underlying index map.
    /// The cached entries are wrapped in a [`Box`] to avoid existing pointers from being
    /// invalidated.
    ///
    /// Note: u32 indices are used over usize to have consistent functionality across architectures.
    /// Some functionality would be different from tests to Wasm if exceeding 32-bit length.
    #[borsh_skip]
    pub(crate) cache: StableMap<u32, OnceCell<CacheEntry<T>>>,
}

impl<T> IndexMap<T>
where
    T: BorshSerialize,
{
    /// Create new index map. Prefixes storage accesss with the prefix provided.
    pub fn new<S>(prefix: S) -> Self
    where
        S: IntoStorageKey,
    {
        Self { prefix: prefix.into_storage_key().into_boxed_slice(), cache: Default::default() }
    }

    fn index_to_lookup_key(prefix: &[u8], index: u32, buf: &mut Vec<u8>) {
        buf.extend_from_slice(prefix);
        buf.extend_from_slice(&index.to_le_bytes());
    }

    /// Flushes the cache and writes all modified values to storage.
    pub fn flush(&mut self) {
        let mut buf = Vec::new();
        // Capacity is prefix length plus bytes needed for u32 bytes (4*u8)
        let mut key_buf = Vec::with_capacity(self.prefix.len() + 4);
        for (k, v) in self.cache.inner().iter_mut() {
            if let Some(v) = v.get_mut() {
                if v.is_modified() {
                    key_buf.clear();
                    Self::index_to_lookup_key(&self.prefix, *k, &mut key_buf);
                    match v.value().as_ref() {
                        Some(modified) => {
                            buf.clear();
                            BorshSerialize::serialize(modified, &mut buf)
                                .unwrap_or_else(|_| env::panic_str(ERR_ELEMENT_SERIALIZATION));
                            env::storage_write(&key_buf, &buf);
                        }
                        None => {
                            // Element was removed, clear the storage for the value
                            env::storage_remove(&key_buf);
                        }
                    }

                    // Update state of flushed state as cached, to avoid duplicate writes/removes
                    // while also keeping the cached values in memory.
                    v.replace_state(EntryState::Cached);
                }
            }
        }
    }

    /// Sets a value at a given index to the value provided. If none is provided, this index will
    /// be removed from storage.
    pub fn set(&mut self, index: u32, value: Option<T>) {
        let entry = self.cache.get_mut(index);
        match entry.get_mut() {
            Some(entry) => *entry.value_mut() = value,
            None => {
                let _ = entry.set(CacheEntry::new_modified(value));
            }
        }
    }
}

impl<T> IndexMap<T>
where
    T: BorshSerialize + BorshDeserialize,
{
    fn deserialize_element(raw_element: &[u8]) -> T {
        T::try_from_slice(raw_element)
            .unwrap_or_else(|_| env::panic_str(ERR_ELEMENT_DESERIALIZATION))
    }

    /// Returns the element by index or `None` if it is not present.
    pub fn get(&self, index: u32) -> Option<&T> {
        let entry = self.cache.get(index).get_or_init(|| {
            let mut buf = Vec::with_capacity(self.prefix.len() + 4);
            Self::index_to_lookup_key(&self.prefix, index, &mut buf);
            let storage_bytes = env::storage_read(&buf);
            let value = storage_bytes.as_deref().map(Self::deserialize_element);
            CacheEntry::new_cached(value)
        });
        entry.value().as_ref()
    }

    /// Returns a mutable reference to the element at the `index` provided.
    pub(crate) fn get_mut_inner(&mut self, index: u32) -> &mut CacheEntry<T> {
        let prefix = &self.prefix;
        let entry = self.cache.get_mut(index);
        entry.get_or_init(|| {
            let mut key = Vec::with_capacity(prefix.len() + 4);
            Self::index_to_lookup_key(prefix, index, &mut key);
            let storage_bytes = env::storage_read(&key);
            let value = storage_bytes.as_deref().map(Self::deserialize_element);
            CacheEntry::new_cached(value)
        });
        let entry = entry.get_mut().unwrap();
        entry
    }

    /// Returns a mutable reference to the element at the `index` provided.
    pub fn get_mut(&mut self, index: u32) -> Option<&mut T> {
        let entry = self.get_mut_inner(index);
        entry.value_mut().as_mut()
    }

    pub fn swap(&mut self, a: u32, b: u32) {
        if a == b {
            // Short circuit if indices are the same, also guarantees uniqueness below
            return;
        }

        let val_a = self.get_mut_inner(a).replace(None);
        let val_b = self.get_mut_inner(b).replace(val_a);
        self.get_mut_inner(a).replace(val_b);
    }

    /// Inserts a element at `index`, returns the evicted element.
    pub fn insert(&mut self, index: u32, element: T) -> Option<T> {
        self.get_mut_inner(index).replace(Some(element))
    }

    /// Removes value at index and returns existing value.
    #[allow(dead_code)]
    pub fn remove(&mut self, index: u32) -> Option<T> {
        self.get_mut_inner(index).replace(None)
    }
}

impl<T> fmt::Debug for IndexMap<T>
where
    T: BorshSerialize + BorshDeserialize + fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("IndexMap").field("prefix", &self.prefix).finish()
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[cfg(test)]
mod tests {
    use super::IndexMap;

    #[test]
    fn basic_usage() {
        let mut map = IndexMap::new(b"v".to_vec());

        map.insert(3, 3u8);
        map.insert(43, 43);
        map.swap(3, 43);
        assert_eq!(map.get(3), Some(&43));
        assert_eq!(map.remove(43), Some(3));

        map.swap(1, 3);
        *map.get_mut(1).unwrap() += 2;
        assert_eq!(map.get(1), Some(&45));

        map.set(0, Some(1));

        map.flush();
        assert_eq!(map.get(0), Some(&1));
    }
}
