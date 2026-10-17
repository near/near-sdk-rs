//! Contract state. `ContractState` is how the `#[near]` wrapper loads and stores the root.
//! With the `unstable` feature, this module also holds storage v2: path-derived keys,
//! nested collections with no prefixes to pick, and a per-call cache.
//!
//! # Attributes
//!
//! - `#[near(contract_state(layout = v2))]` on the contract struct makes it a v2 root, a
//!   tagged record at key `0xF0` (`root = 0xF1` picks another root byte). Every field needs
//!   `#[key(n)]`.
//! - `#[near(storage)]` on a struct makes it a record: a value that can sit in a map and own
//!   collections. Every field needs `#[key(n)]`; `reserved(3, 7)` blocks retired tags.
//! - `#[key(n)]` is the field's tag. **Tags are the layout: never reuse or renumber one.** A
//!   field missing from a stored record decodes as its `Default`. `#[key(n, required)]` makes
//!   it a decode error instead; use it only for fields present since the first write (a field
//!   added later must be `Default` or `Option`).
//!
//! # What the derive emits for a record `Name`
//!
//! - `impl Default`, unless a field is `required` or you derive one.
//! - `impl Removable` when every field is removable: leaves, `Map`, `Set`, `Vector`, `Lazy`,
//!   and records built from them. A `LookupMap`/`LookupSet`/`GroupedMap` field makes the
//!   record non-removable (they do not track their keys).
//! - `NameAt<'a>` (from `at`) and `NameAtMut<'a>` (from `at_mut`): views whose collection
//!   fields are handles and whose scalars are unavailable.
//! - On the root: `Exchange::bind(self) -> Self`, for an `#[init]` body that seeds collections.
//!
//! # Collections
//!
//! `Map` / `Set` are iterable, pageable and removable (4 writes on a new key); `LookupMap` /
//! `LookupSet` are the opt-in for 1 write per insert. `Vector` is dense slots. `Lazy` is one
//! value at its own key, read only when touched. `GroupedMap` is one `Map` per group key. A map
//! value cannot itself be a collection: put it in a record, or use `GroupedMap`.
//!
//! - `map.get(&k)` reads and decodes the record. `map.at(&k)` walks to it without reading
//!   (read-only). `map.at_mut(&k)` walks to it to write below it: 1 `storage_has_key`, panics
//!   if absent. `entry(k).or_default()` / `get_mut(&k)` give the decoded record to change.
//! - Each key is read at most once per call, and written once at the end, in key order, with the
//!   root last. A record is rewritten only if its encoding changed. `contains_key` is a
//!   `storage_has_key` and does not warm the cache for a later `get`.
//! - `remove` clears the value's collections and returns it detached. `insert` of a record
//!   that owns collections panics if the key exists.
//! - Enumeration order is the key vector's: insertion order until a remove moves the last key
//!   into the hole. `page(after, n)` / `page_with` panic if the cursor key was removed since the
//!   last page, so a public listing that may race with removals should use offset paging,
//!   `iter().skip(from).take(n)`, where `skip` reads nothing.
//!
//! See `state::testing` for the counting mock host and a call harness.

use borsh::{BorshDeserialize, BorshSerialize};

use crate::env;

pub trait ContractState {
    #[inline]
    fn state_key() -> &'static [u8] {
        b"STATE"
    }

    #[inline]
    fn state_exists() -> bool {
        env::storage_has_key(Self::state_key())
    }

    #[inline]
    #[track_caller]
    fn state_read() -> Option<Self>
    where
        Self: StateRead,
    {
        <Self as StateRead>::read_state(Self::state_key())
    }

    /// Writes the contract state. The `#[near]` wrapper calls this after a mutating method.
    ///
    /// For a storage v2 root (`contract_state(layout = v2)`) this flushes the per-call cache
    /// and then frees every cached record. Any `&V` or `&mut V` obtained from a collection
    /// before the call dangles afterwards. Borrowck does not catch it, because `state_write`
    /// takes `&self` and the reference is a shared borrow of the same root. The wrapper never
    /// does this (it writes after the method body returns). Code that calls `state_write()` by
    /// hand in the middle of a method must drop every reference it read first, and re-read
    /// through the collections afterwards. In debug builds this is not checked; Miri catches it.
    #[inline]
    #[track_caller]
    fn state_write(&self) -> bool
    where
        Self: StateWrite,
    {
        <Self as StateWrite>::write_state(self, Self::state_key())
    }
}

/// How a contract root is loaded by the `#[near]` wrapper.
///
/// Every Borsh type gets the v1 behaviour (one blob under `state_key()`) through the blanket
/// impl. A storage v2 root (`contract_state(layout = v2)`) is not Borsh and gets a generated
/// impl instead. This keeps the wrapper's generated code identical for v1 and v2 contracts.
pub trait StateRead: Sized {
    fn read_state(key: &[u8]) -> Option<Self>;
}

/// How a contract root is stored by the `#[near]` wrapper. See [`StateRead`].
pub trait StateWrite {
    fn write_state(&self, key: &[u8]) -> bool;
}

impl<T: BorshDeserialize> StateRead for T {
    #[inline]
    #[track_caller]
    fn read_state(key: &[u8]) -> Option<Self> {
        env::storage_read(key).map(|data| {
            borsh::from_slice(&data)
                .unwrap_or_else(|_| env::panic_str("Cannot deserialize the contract state."))
        })
    }
}

impl<T: BorshSerialize> StateWrite for T {
    #[inline]
    #[track_caller]
    fn write_state(&self, key: &[u8]) -> bool {
        env::storage_write(
            key,
            &borsh::to_vec(self)
                .unwrap_or_else(|_| env::panic_str("Cannot serialize the contract state.")),
        )
    }
}

// ---- storage v2 prototype (see PROTOTYPE_NOTES.md) ----

#[cfg(feature = "unstable")]
mod cache;
#[cfg(feature = "unstable")]
mod collections;
#[cfg(feature = "unstable")]
pub mod key;
#[cfg(feature = "unstable")]
pub mod record;
#[cfg(feature = "unstable")]
pub mod testing;
#[cfg(feature = "unstable")]
mod value;
#[cfg(feature = "unstable")]
mod root;
#[cfg(feature = "unstable")]
#[cfg(test)]
mod tests;

#[cfg(feature = "unstable")]
pub use collections::{
    At, Group, GroupMut, GroupedMap, IndexIter, KeyPage, Lazy, LookupMap, LookupMapEntry,
    LookupSet, Map, MapEntry, Page, Set, Vector, ViewIsReadOnly,
};
#[cfg(feature = "unstable")]
pub use key::{Path, StorageKey};
#[cfg(feature = "unstable")]
pub use record::RecordError;
#[cfg(feature = "unstable")]
pub use root::{Root, __private, flush_root, load_root, raw_root, root_exists};
#[cfg(feature = "unstable")]
pub use value::{
    FieldDefault, Record, Removable, UseGetForScalars, UseGetMutForScalars, Value, ViewField,
    ViewFieldMut,
};
