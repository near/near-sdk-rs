mod impls;

use borsh::{BorshDeserialize, BorshSerialize};
use once_cell::unsync::OnceCell;

use crate::env;
use crate::store::lazy::{load_and_deserialize, serialize_and_store};
use crate::utils::{CacheEntry, EntryState};
use crate::IntoStorageKey;

/// An persistent lazily loaded option, that stores a `value` in the storage when `Some(value)`
/// is set, and not when `None` is set. `LazyOption` also [`Deref`]s into [`Option`] so we get
/// all its APIs for free.
///
/// This will only write to the underlying store if the value has changed, and will only read the
/// existing value from storage once.
///
/// # Examples
/// ```
/// use near_sdk::store::LazyOption;
///
/// let mut a = LazyOption::new(b"a", None);
/// assert!(a.is_none());
///
/// *a = Some("new value".to_owned());
/// assert_eq!(a.get(), &Some("new value".to_owned()));
///
/// // Using Option::replace:
/// let old_str = a.replace("new new value".to_owned());
/// assert_eq!(old_str, Some("new value".to_owned()));
/// assert_eq!(a.get(), &Some("new new value".to_owned()));
/// ```
/// [`Deref`]: std::ops::Deref
#[derive(BorshSerialize, BorshDeserialize)]
pub struct LazyOption<T>
where
    T: BorshSerialize,
{
    /// Key bytes to index the contract's storage.
    storage_key: Box<[u8]>,

    /// Cached value which is lazily loaded and deserialized from storage.
    #[borsh_skip]
    cache: OnceCell<CacheEntry<T>>,
}

impl<T> LazyOption<T>
where
    T: BorshSerialize,
{
    /// Create a new lazy option with the given `storage_key` and the initial value.
    pub fn new<S>(storage_key: S, value: Option<T>) -> Self
    where
        S: IntoStorageKey,
    {
        let cache = match value {
            Some(value) => CacheEntry::new_modified(Some(value)),
            None => CacheEntry::new_cached(None),
        };

        Self {
            storage_key: storage_key.into_storage_key().into_boxed_slice(),
            cache: OnceCell::from(cache),
        }
    }

    /// Updates the value with a new value. This does not load the current value from storage.
    pub fn set(&mut self, value: Option<T>) {
        if let Some(v) = self.cache.get_mut() {
            *v.value_mut() = value;
        } else {
            self.cache
                .set(CacheEntry::new_modified(value))
                .unwrap_or_else(|_| env::panic_str("cache is checked to not be filled above"));
        }
    }

    /// Writes any changes to the value to storage. This will automatically be done when the
    /// value is dropped through [`Drop`] so this should only be used when the changes need to be
    /// reflected in the underlying storage before then.
    pub fn flush(&mut self) {
        if let Some(v) = self.cache.get_mut() {
            if !v.is_modified() {
                return;
            }

            match v.value().as_ref() {
                Some(value) => serialize_and_store(&self.storage_key, value),
                None => {
                    env::storage_remove(&self.storage_key);
                }
            }

            // Replaces cache entry state to cached because the value in memory matches the
            // stored value. This avoids writing the same value twice.
            v.replace_state(EntryState::Cached);
        }
    }
}

impl<T> LazyOption<T>
where
    T: BorshSerialize + BorshDeserialize,
{
    /// Returns a reference to the lazily loaded optional.
    /// The load from storage only happens once, and if the value is already cached, it will not
    /// be reloaded.
    pub fn get(&self) -> &Option<T> {
        let entry = self.cache.get_or_init(|| load_and_deserialize(&self.storage_key));
        entry.value()
    }

    /// Returns a reference to the lazily loaded optional.
    /// The load from storage only happens once, and if the value is already cached, it will not
    /// be reloaded.
    pub fn get_mut(&mut self) -> &mut Option<T> {
        self.cache.get_or_init(|| load_and_deserialize(&self.storage_key));
        let entry = self.cache.get_mut().unwrap_or_else(|| env::abort());
        entry.value_mut()
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    pub fn test_lazy_option() {
        let mut a = LazyOption::new(b"a", None);
        assert!(a.is_none());
        assert!(!env::storage_has_key(b"a"));

        // Check value has been set in via cache:
        a.set(Some(42u32));
        assert!(a.is_some());
        assert_eq!(a.get(), &Some(42));

        // Flushing, then check if storage has been set:
        a.flush();
        assert!(env::storage_has_key(b"a"));
        assert_eq!(u32::try_from_slice(&env::storage_read(b"a").unwrap()).unwrap(), 42);

        // New value is set
        *a = Some(49u32);
        assert!(a.is_some());
        assert_eq!(a.get(), &Some(49));

        // Testing `Option::replace`
        let old = a.replace(69u32);
        assert!(a.is_some());
        assert_eq!(old, Some(49));

        // Testing `Option::take` deletes from internal storage
        let taken = a.take();
        assert!(a.is_none());
        assert_eq!(taken, Some(69));

        // `flush`/`drop` after `Option::take` should remove from storage:
        drop(a);
        assert!(!env::storage_has_key(b"a"));
    }
}
