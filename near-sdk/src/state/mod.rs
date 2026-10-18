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
//!   field missing from a stored record decodes as its `Default`. `#[key(n, default = expr)]`
//!   evaluates `expr` instead (serde-style, D2; the type needs no `Default`, and the generated
//!   `Default` uses it too). `#[key(n, required)]` makes it a decode error naming the tag; use
//!   it only for fields present since the first write (a field added later uses `default = ..`,
//!   a `Default` type, or `Option`; never forced `Option`). `required` and `default` are
//!   exclusive. The deployed-ABI `storage diff` check that hard-fails a newly added `required`
//!   tag is not built in the prototype.
//! - Unknown tags (D4): a tag the code does not know is PRESERVED through `get_mut` / `entry`
//!   (a blind `insert` replaces the record whole). A removed field's bytes therefore persist
//!   until its tag is listed in `reserved(n)`, which drops them on the next rewrite. Reusing a
//!   removed tag for a new field resurrects the old bytes (pinned by
//!   `hazard_reusing_a_removed_tag_resurrects_its_stale_bytes`); the intended guard, a build
//!   error against the deployed ABI, is not built in the prototype.
//!
//! # What the derive emits for a record `Name`
//!
//! - `impl Default`, unless a field is `required` or you derive one.
//! - `impl Removable` when every field is removable: leaves, `Map`, `Set`, `Vector`, `Lazy`,
//!   and records built from them. A `LookupMap`/`LookupSet` field makes the record
//!   non-removable (they do not track their keys).
//! - `impl Leaf` when no field owns storage (D6): only then is plain `insert` available.
//! - `NameAt<'a>` (from `at`) and `NameAtMut<'a>` (from `at_mut`): views whose collection
//!   fields are handles and whose scalars are unavailable.
//! - On the root: `Exchange::bind(self) -> Self`, for an `#[init]` body that seeds collections.
//!
//! # Collections
//!
//! `Map` / `Set` are iterable, pageable and removable (4 writes on a new key); `LookupMap` /
//! `LookupSet` are the opt-in for 1 write per insert. `Vector` is dense slots. `Lazy` is one
//! value at its own key, read only when touched. A map value cannot itself be a collection: put
//! it in a record (`Map<MarketId, Market>` with `Market { orders: Map<OrderId, Order>, .. }`).
//! (`GroupedMap` is deferred out of the first release, D3; it is `#[doc(hidden)]` and kept only
//! so the prototype's NFT test compiles.)
//!
//! - `map.get(&k)` reads and decodes the record. `map.at(&k)` walks to it without reading
//!   (read-only). `map.at_mut(&k)` walks to it to write below it: 1 read of the record's bytes,
//!   not decoded (D9: it must see the clearing flag; it was 1 `storage_has_key` before), and
//!   `None` if absent (D7: named accessors return `Option`; only `Index` would panic).
//!   `entry(k).or_default()` / `get_mut(&k)` give the decoded record to change.
//! - Each key is read at most once per call, and written once at the end, in key order, with the
//!   root last. A record is rewritten only if its encoding changed. `contains_key` is a
//!   `storage_has_key` and does not warm the cache for a later `get`.
//! - `remove` clears the value's collections and returns it detached. `remove_some(&k, n)`
//!   (D9) removes a record in bounded steps across calls: at most `n` entries of what it owns
//!   per call, then the record; `true` while entries remain. Meanwhile the record carries the
//!   clearing flag and `get_mut`, `entry` and `at_mut` on it panic, so nothing can be added
//!   below it faster than it is cleared (`get` / `at` still read).
//! - FORMAT DECISION (D9): the clearing flag is the field `00 00` (tag 0, length 0) at the front
//!   of the record bytes. Two bytes is the smallest field the grammar allows; tag 0 is never a
//!   user tag, so it cannot collide, and tags ascend, so it can only be first. A decoder that
//!   predates it rejects the record with `TagZero` (fails closed). See `record::CLEARING_FLAG`.
//! - `insert` always overwrites (D6). `Map::insert` returns whether the key was new (free, from
//!   the index read); `LookupMap::insert` returns `()` (no existence read). It requires
//!   `V: Leaf`, a value that owns no collections, so for a record that owns one plain `insert`
//!   does not compile (E0277 naming the fix). The ways in are `insert_new(k, v)` (`V: Record`;
//!   panics if present, returns the bound record) and `entry(k).or_default()`. First-write-wins
//!   is `entry(k).or_insert(v)` or `insert_new`.
//! - Enumeration order is the key vector's: insertion order until a remove moves the last key
//!   into the hole. `page(after, n)` / `page_with` / `keys_page` return `Err(CursorGone)` if the
//!   cursor key was removed since the last page (D8); a `#[handle_result]` view can return the
//!   `Result` as is. Offset paging, `iter().skip(from).take(n)` (where `skip` reads nothing),
//!   never fails but can skip a key after a removal behind the offset. An on-chain batch job
//!   must not page a map that changes between its calls.
//!
//! - Storage accounting (D5): v2 writes land at the flush, after the method body, so
//!   `env::storage_usage()` read inside a v2 mutating method after its first write measures
//!   zero change. Under `unit-testing` that read panics, pointing at `storage_delta()`; the
//!   post-flush value is `Receipt::storage_delta()` / `testing::storage_delta()` (key + value +
//!   40 bytes per new record). The on-chain `env::storage_delta()` is not built.
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
mod migrating;
#[cfg(feature = "unstable")]
#[cfg(test)]
mod tests;
/// Reference model of the v2 layer (keys, records, collections, per-call cache, cost formulas),
/// the oracle the `verify` harnesses compare the implementation against.
#[cfg(feature = "unstable")]
#[cfg(test)]
mod model;
/// Bolero / Kani harnesses for the seven guarantees and the named bug classes.
#[cfg(feature = "unstable")]
#[cfg(test)]
mod verify;

#[cfg(feature = "unstable")]
pub use collections::{
    At, CursorGone, IndexIter, KeyPage, Lazy, LookupMap, LookupMapEntry, LookupSet, Map, MapEntry,
    Page, Set, Vector, ViewIsReadOnly,
};
/// Deferred out of the first release (D3); hidden, kept compiling for the prototype.
#[cfg(feature = "unstable")]
#[doc(hidden)]
pub use collections::{Group, GroupMut, GroupedMap};
#[cfg(feature = "unstable")]
pub use key::{Path, StorageKey};
#[cfg(feature = "unstable")]
pub use migrating::{LegacyIterable, LegacyMap, MigrationTarget, Migrating, V1PrefixError, check_v1_prefix};
#[cfg(feature = "unstable")]
pub use record::RecordError;
#[cfg(feature = "unstable")]
pub use root::{Root, __private, flush_root, load_root, raw_root, root_exists};
#[cfg(feature = "unstable")]
pub use value::{
    FieldDefault, Leaf, Record, Removable, UseGetForScalars, UseGetMutForScalars, Value, ViewField,
    ViewFieldMut,
};
