//! A persistent lazy option. Stores a value for a given key.
//! Example:
//! If the underlying value is large, e.g. the contract needs to store an image, but it doesn't need
//! to have access to this image at regular calls, then the contract can wrap this image into
//! `LazyOption` and it will not be deserialized until requested.
use std::marker::PhantomData;

use borsh::{BorshDeserialize, BorshSerialize};

use crate::env;
use crate::IntoStorageKey;

const ERR_VALUE_SERIALIZATION: &str = "Cannot serialize value with Borsh";
const ERR_VALUE_DESERIALIZATION: &str = "Cannot deserialize value with Borsh";

/// An persistent lazy option, that stores a value in the storage.
#[derive(BorshSerialize, BorshDeserialize)]
pub struct LazyOption<T> {
    storage_key: Vec<u8>,
    #[borsh_skip]
    el: PhantomData<T>,
}

impl<T> LazyOption<T> {
    /// Returns `true` if the value is present in the storage.
    pub fn is_some(&self) -> bool {
        env::storage_has_key(&self.storage_key)
    }

    /// Returns `true` if the value is not present in the storage.
    pub fn is_none(&self) -> bool {
        !self.is_some()
    }

    /// Reads the raw value from the storage
    fn get_raw(&self) -> Option<Vec<u8>> {
        env::storage_read(&self.storage_key)
    }

    /// Removes the value from the storage.
    /// Returns true if the element was present.
    fn remove_raw(&mut self) -> bool {
        env::storage_remove(&self.storage_key)
    }

    /// Removes the raw value from the storage and returns it as an option.
    fn take_raw(&mut self) -> Option<Vec<u8>> {
        if self.remove_raw() {
            Some(env::storage_get_evicted().unwrap())
        } else {
            None
        }
    }

    fn set_raw(&mut self, raw_value: &[u8]) -> bool {
        env::storage_write(&self.storage_key, raw_value)
    }

    fn replace_raw(&mut self, raw_value: &[u8]) -> Option<Vec<u8>> {
        if self.set_raw(raw_value) {
            Some(env::storage_get_evicted().unwrap())
        } else {
            None
        }
    }
}

impl<T> LazyOption<T>
where
    T: BorshSerialize + BorshDeserialize,
{
    /// Create a new lazy option with the given `storage_key` and the initial value.
    pub fn new<S>(storage_key: S, value: Option<&T>) -> Self
    where
        S: IntoStorageKey,
    {
        let mut this = Self { storage_key: storage_key.into_storage_key(), el: PhantomData };
        if let Some(value) = value {
            this.set(value);
        }
        this
    }

    fn serialize_value(value: &T) -> Vec<u8> {
        match value.try_to_vec() {
            Ok(x) => x,
            Err(_) => env::panic_str(ERR_VALUE_SERIALIZATION),
        }
    }

    fn deserialize_value(raw_value: &[u8]) -> T {
        match T::try_from_slice(raw_value) {
            Ok(x) => x,
            Err(_) => env::panic_str(ERR_VALUE_DESERIALIZATION),
        }
    }

    /// Removes the value from storage without reading it.
    /// Returns whether the value was present.
    pub fn remove(&mut self) -> bool {
        self.remove_raw()
    }

    /// Removes the value from storage and returns it as an option.
    pub fn take(&mut self) -> Option<T> {
        self.take_raw().map(|v| Self::deserialize_value(&v))
    }

    /// Gets the value from storage and returns it as an option.
    pub fn get(&self) -> Option<T> {
        self.get_raw().map(|v| Self::deserialize_value(&v))
    }

    /// Sets the value into the storage without reading the previous value and returns whether the
    /// previous value was present.
    pub fn set(&mut self, value: &T) -> bool {
        self.set_raw(&Self::serialize_value(value))
    }

    /// Replaces the value in the storage and returns the previous value as an option.
    pub fn replace(&mut self, value: &T) -> Option<T> {
        self.replace_raw(&Self::serialize_value(value)).map(|v| Self::deserialize_value(&v))
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    pub fn test_all() {
        let mut a = LazyOption::new(b"a", None);
        assert!(a.is_none());
        a.set(&42u32);
        assert!(a.is_some());
        assert_eq!(a.get(), Some(42));
        assert!(a.is_some());
        assert_eq!(a.replace(&95), Some(42));
        assert!(a.is_some());
        assert_eq!(a.take(), Some(95));
        assert!(a.is_none());
        assert_eq!(a.replace(&105), None);
        assert!(a.is_some());
        assert_eq!(a.get(), Some(105));
        assert!(a.remove());
        assert!(a.is_none());
        assert_eq!(a.get(), None);
        assert_eq!(a.take(), None);
        assert!(a.is_none());
    }

    #[test]
    pub fn test_multi() {
        let mut a = LazyOption::new(b"a", None);
        let mut b = LazyOption::new(b"b", None);
        assert!(a.is_none());
        assert!(b.is_none());
        a.set(&42u32);
        assert!(b.is_none());
        assert!(a.is_some());
        assert_eq!(a.get(), Some(42));
        b.set(&32u32);
        assert!(a.is_some());
        assert!(b.is_some());
        assert_eq!(a.get(), Some(42));
        assert_eq!(b.get(), Some(32));
    }

    #[test]
    pub fn test_init_value() {
        let a = LazyOption::new(b"a", Some(&42u32));
        assert!(a.is_some());
        assert_eq!(a.get(), Some(42));
    }
}
