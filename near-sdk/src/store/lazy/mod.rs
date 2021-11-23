//! A persistent lazy storage value. Stores a value for a given key.
//! Example:
//! If the underlying value is large, e.g. the contract needs to store an image, but it doesn't need
//! to have access to this image at regular calls, then the contract can wrap this image into
//! [`Lazy`] and it will not be deserialized until requested.

mod impls;

use borsh::{BorshDeserialize, BorshSerialize};
use once_cell::unsync::OnceCell;

use crate::env;
use crate::utils::{CacheEntry, EntryState};
use crate::IntoStorageKey;

const ERR_VALUE_SERIALIZATION: &str = "Cannot serialize value with Borsh";
const ERR_VALUE_DESERIALIZATION: &str = "Cannot deserialize value with Borsh";
const ERR_NOT_FOUND: &str = "No value found for the given key";
const ERR_DELETED: &str = "The Lazy cell's value has been deleted. Verify the key has not been\
                            deleted manually.";

fn expect_key_exists<T>(val: Option<T>) -> T {
    val.unwrap_or_else(|| env::panic_str(ERR_NOT_FOUND))
}

fn expect_consistent_state<T>(val: Option<T>) -> T {
    val.unwrap_or_else(|| env::panic_str(ERR_DELETED))
}

pub(crate) fn load_and_deserialize<T>(key: &[u8]) -> CacheEntry<T>
where
    T: BorshDeserialize,
{
    let bytes = expect_key_exists(env::storage_read(key));
    let val =
        T::try_from_slice(&bytes).unwrap_or_else(|_| env::panic_str(ERR_VALUE_DESERIALIZATION));
    CacheEntry::new_cached(Some(val))
}

pub(crate) fn serialize_and_store<T>(key: &[u8], value: &T)
where
    T: BorshSerialize,
{
    let serialized = value.try_to_vec().unwrap_or_else(|_| env::panic_str(ERR_VALUE_SERIALIZATION));
    env::storage_write(key, &serialized);
}

/// An persistent lazily loaded value, that stores a value in the storage.
///
/// This will only write to the underlying store if the value has changed, and will only read the
/// existing value from storage once.
///
/// # Examples
/// ```
/// use near_sdk::store::Lazy;
///
/// let mut a = Lazy::new(b"a", "test string".to_string());
/// assert_eq!(*a, "test string");
///
/// *a = "new string".to_string();
/// assert_eq!(a.get(), "new string");
/// ```
#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct Lazy<T>
where
    T: BorshSerialize,
{
    /// Key bytes to index the contract's storage.
    storage_key: Box<[u8]>,
    #[borsh_skip]
    /// Cached value which is lazily loaded and deserialized from storage.
    cache: OnceCell<CacheEntry<T>>,
}

impl<T> Lazy<T>
where
    T: BorshSerialize,
{
    pub fn new<S>(key: S, value: T) -> Self
    where
        S: IntoStorageKey,
    {
        Self {
            storage_key: key.into_storage_key().into_boxed_slice(),
            cache: OnceCell::from(CacheEntry::new_modified(Some(value))),
        }
    }

    /// Updates the value with a new value. This does not load the current value from storage.
    pub fn set(&mut self, value: T) {
        if let Some(v) = self.cache.get_mut() {
            *v.value_mut() = Some(value);
        } else {
            self.cache
                .set(CacheEntry::new_modified(Some(value)))
                .unwrap_or_else(|_| env::panic_str("cache is checked to not be filled above"))
        }
    }

    /// Writes any changes to the value to storage. This will automatically be done when the
    /// value is dropped through [`Drop`] so this should only be used when the changes need to be
    /// reflected in the underlying storage before then.
    pub fn flush(&mut self) {
        if let Some(v) = self.cache.get_mut() {
            if v.is_modified() {
                // Value was modified, serialize and put the serialized bytes in storage.
                let value = expect_consistent_state(v.value().as_ref());
                serialize_and_store(&self.storage_key, value);

                // Replaces cache entry state to cached because the value in memory matches the
                // stored value. This avoids writing the same value twice.
                v.replace_state(EntryState::Cached);
            }
        }
    }
}

impl<T> Lazy<T>
where
    T: BorshSerialize + BorshDeserialize,
{
    /// Returns a reference to the lazily loaded storage value.
    /// The load from storage only happens once, and if the value is already cached, it will not
    /// be reloaded.
    ///
    /// This function will panic if the cache is not loaded and the value at the key does not exist.
    pub fn get(&self) -> &T {
        let entry = self.cache.get_or_init(|| load_and_deserialize(&self.storage_key));

        expect_consistent_state(entry.value().as_ref())
    }

    /// Returns a reference to the lazily loaded storage value.
    /// The load from storage only happens once, and if the value is already cached, it will not
    /// be reloaded.
    ///
    /// This function will panic if the cache is not loaded and the value at the key does not exist.
    pub fn get_mut(&mut self) -> &mut T {
        self.cache.get_or_init(|| load_and_deserialize(&self.storage_key));
        let entry = self.cache.get_mut().unwrap_or_else(|| env::abort());

        expect_consistent_state(entry.value_mut().as_mut())
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    pub fn test_lazy() {
        let mut a = Lazy::new(b"a", 8u32);
        assert_eq!(a.get(), &8);

        assert!(!env::storage_has_key(b"a"));
        a.flush();
        assert_eq!(u32::try_from_slice(&env::storage_read(b"a").unwrap()).unwrap(), 8);

        a.set(42);

        // Value in storage will still be 8 until the value is flushed
        assert_eq!(u32::try_from_slice(&env::storage_read(b"a").unwrap()).unwrap(), 8);
        assert_eq!(*a, 42);

        *a = 30;
        let serialized = a.try_to_vec().unwrap();
        drop(a);
        assert_eq!(u32::try_from_slice(&env::storage_read(b"a").unwrap()).unwrap(), 30);

        let lazy_loaded = Lazy::<u32>::try_from_slice(&serialized).unwrap();
        assert!(lazy_loaded.cache.get().is_none());

        let b = Lazy::new(b"b", 30);
        assert!(!env::storage_has_key(b"b"));

        // A value that is not stored in storage yet and one that has not been loaded yet can
        // be checked for equality.
        assert_eq!(lazy_loaded, b);
    }
}
