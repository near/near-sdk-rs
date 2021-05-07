//! A persistent lazy storage value. Stores a value for a given key.
//! Example:
//! If the underlying value is large, e.g. the contract needs to store an image, but it doesn't need
//! to have access to this image at regular calls, then the contract can wrap this image into
//! [`Lazy`] and it will not be deserialized until requested.

mod impls;

use borsh::{BorshDeserialize, BorshSerialize};
use once_cell::unsync::OnceCell;

use crate::env;
use crate::IntoStorageKey;

const ERR_VALUE_SERIALIZATION: &[u8] = b"Cannot serialize value with Borsh";
const ERR_VALUE_DESERIALIZATION: &[u8] = b"Cannot deserialize value with Borsh";
const ERR_NOT_FOUND: &[u8] = b"No value found for the given key";

fn expect_key_exists<T>(val: Option<T>) -> T {
    val.unwrap_or_else(|| env::panic(ERR_NOT_FOUND))
}

fn load_and_deserialize<T>(key: &[u8]) -> T
where
    T: BorshDeserialize,
{
    let bytes = expect_key_exists(env::storage_read(key));
    T::try_from_slice(&bytes).unwrap_or_else(|_| env::panic(ERR_VALUE_DESERIALIZATION))
}

/// An persistent lazily loaded value, that stores a value in the storage.
#[derive(BorshSerialize, BorshDeserialize)]
pub struct Lazy<T>
where
    T: BorshSerialize,
{
    /// Key bytes to index the contract's storage.
    storage_key: Vec<u8>,
    #[borsh_skip]
    /// Cached value which is lazily loaded and deserialized from storage.
    cache: OnceCell<T>,
}

impl<T> Lazy<T>
where
    T: BorshSerialize,
{
    pub fn new<S>(key: S, value: T) -> Self
    where
        S: IntoStorageKey,
    {
        Self { storage_key: key.into_storage_key(), cache: OnceCell::from(value) }
    }

    /// Updates the value with a new value. This does not load the current value from storage.
    pub fn set(&mut self, value: T) {
        if let Some(v) = self.cache.get_mut() {
            *v = value;
        } else {
            self.cache.set(value).ok().expect("cache is checked to not be filled above");
        }
    }

    /// Writes any changes to the value to storage. This will automatically be done when the
    /// value is dropped through [`Drop`] so this should only be used when the changes need to be
    /// reflected in the underlying storage before then.
    pub fn flush(&mut self) {
        // TODO
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
        self.cache.get_or_init(|| load_and_deserialize(&self.storage_key))
    }

    /// Returns a reference to the lazily loaded storage value.
    /// The load from storage only happens once, and if the value is already cached, it will not
    /// be reloaded.
    ///
    /// This function will panic if the cache is not loaded and the value at the key does not exist.
    pub fn get_mut(&mut self) -> &mut T {
        self.cache.get_or_init(|| load_and_deserialize(&self.storage_key));
        self.cache.get_mut().expect("cell should be filled above")
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[cfg(test)]
mod tests {
    use super::*;

    use crate::test_utils::test_env;

    #[test]
    pub fn test_all() {
        // test_env::setup();
        // let mut a = Lazy::new(b"a", None);
        // assert!(a.is_none());
        // a.set(&42u32);
        // assert!(a.is_some());
        // assert_eq!(a.get(), Some(42));
        // assert!(a.is_some());
        // assert_eq!(a.replace(&95), Some(42));
        // assert!(a.is_some());
        // assert_eq!(a.take(), Some(95));
        // assert!(a.is_none());
        // assert_eq!(a.replace(&105), None);
        // assert!(a.is_some());
        // assert_eq!(a.get(), Some(105));
        // assert!(a.remove());
        // assert!(a.is_none());
        // assert_eq!(a.get(), None);
        // assert_eq!(a.take(), None);
        // assert!(a.is_none());
    }

    #[test]
    pub fn test_multi() {
        // test_env::setup();
        // let mut a = Lazy::new(b"a", None);
        // let mut b = Lazy::new(b"b", None);
        // assert!(a.is_none());
        // assert!(b.is_none());
        // a.set(&42u32);
        // assert!(b.is_none());
        // assert!(a.is_some());
        // assert_eq!(a.get(), Some(42));
        // b.set(&32u32);
        // assert!(a.is_some());
        // assert!(b.is_some());
        // assert_eq!(a.get(), Some(42));
        // assert_eq!(b.get(), Some(32));
    }

    #[test]
    pub fn test_init_value() {
        // test_env::setup();
        // let a = Lazy::new(b"a", Some(&42u32));
        // assert!(a.is_some());
        // assert_eq!(a.get(), Some(42));
    }
}
