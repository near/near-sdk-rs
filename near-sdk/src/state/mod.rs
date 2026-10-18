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
//!   uses `expr` instead (serde-style, D2; the type needs no `Default`, and the generated
//!   `Default` uses it too). `expr` must be a const expression (T6): the derive emits it as a
//!   `const` item, so a call like `env::predecessor_account_id()` is a compile error, and the
//!   value an old record decodes to cannot depend on who reads it or when. `AccountId` has no
//!   const constructor, so a later-added `AccountId` field is `Option<AccountId>`. `#[key(n, required)]` makes it a decode error naming the tag; use
//!   it only for fields present since the first write (a field added later uses `default = ..`,
//!   a `Default` type, or `Option`; never forced `Option`). `required` and `default` are
//!   exclusive. The deployed-ABI `storage diff` check that hard-fails a newly added `required`
//!   tag is not built in the prototype.
//! - Field types (T7): every field type must implement [`Value`]. There is no blanket impl
//!   for Borsh types: primitives, `String`, `AccountId`, `Vec`/`Option`/arrays/tuples of
//!   those, `state` collections and `#[near(storage)]` records are values, and a plain Borsh
//!   struct opts in with `#[derive(Leaf)]` ([`BorshLeaf`]). v1 collections (`store::*`,
//!   `collections::*`, and standards types holding one) implement nothing, so inside a record
//!   they are E0277 "is not a storage v2 value", a shadowing `use near_sdk::store::LookupMap`
//!   included. At the root they are allowed as `#[key(n, legacy)]`: stored as their Borsh
//!   bytes (the prefix) outside the trait, required, never bound; `legacy` anywhere else is a
//!   derive error.
//! - Unknown tags (D4): a tag the code does not know is PRESERVED through `get_mut` / `entry`
//!   (a blind `insert` replaces the record whole). A removed field's bytes therefore persist
//!   until its tag is listed in `reserved(n)`, which drops them on the next rewrite. Reusing a
//!   removed tag for a new field resurrects the old bytes (pinned by
//!   `hazard_reusing_a_removed_tag_resurrects_its_stale_bytes`); the intended guard, a build
//!   error against the deployed ABI, is not built in the prototype.
//!
//! A non-const default does not compile:
//!
//! ```compile_fail,E0015
//! use near_sdk::{env, near, AccountId};
//! #[near(storage)]
//! pub struct Pool {
//!     #[key(1)] fee_bps: u16,
//!     // error[E0015]: cannot call non-const function
//!     // `near_sdk::env::predecessor_account_id` in constants
//!     #[key(2, default = env::predecessor_account_id())] owner: AccountId,
//! }
//! ```
//!
//! A const one does:
//!
//! ```
//! use near_sdk::near;
//! pub const DEFAULT_FEE: u16 = 30;
//! #[near(storage)]
//! pub struct Pool {
//!     #[key(1, default = DEFAULT_FEE)] fee_bps: u16,
//!     #[key(2, default = true)] open: bool,
//! }
//! assert_eq!((Pool::default().fee_bps, Pool::default().open), (30, true));
//! ```
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
//!   (read-only). `map.at_mut(&k)` walks to it to write below it: 1 `storage_has_key`, no read,
//!   no decode, and `None` if absent (D7: named accessors return `Option`; only `Index` would
//!   panic). An absent record costs a 2nd check, of its D9 flag key.
//!   `entry(k).or_default()` / `get_mut(&k)` give the decoded record to change.
//! - Each key is read at most once per call, and written once at the end, in key order, with the
//!   root last. A record is rewritten only if its encoding changed. `contains_key` is a
//!   `storage_has_key` and does not warm the cache for a later `get`.
//! - `remove` clears the value's collections and returns it detached. `remove_step(&k, n)`
//!   (D9; round 3 T3) removes a record in bounded steps across calls: at most `n` entries of
//!   what it owns per call, then the record; it returns the `#[must_use]` enum `Removal`,
//!   `Pending` while entries remain and `Done` once the record is gone. Meanwhile the record is flagged and
//!   `get_mut`, `entry` and `at_mut` on it panic, so nothing can be added below it faster than
//!   it is cleared (`get` / `at` still read).
//! - FORMAT DECISION (D9, flag key): the clearing flag is a separate key, the record's key plus
//!   one `00` byte (`Path::flag`; tag 0, which no field may use). While a record is being
//!   cleared its bytes live at the flag key and its record key is absent; the call that
//!   finishes removes the flag key. So presence of the record key means "present and not
//!   being cleared": `at_mut`, `get_mut` and `entry` on a present record pay nothing for D9,
//!   and only a miss on a map value that owns storage consults the flag key (1 more host
//!   call). Record bytes carry no flag; tag 0 in a record is always `TagZero`.
//! - `insert` always overwrites (D6). `Map::insert` returns whether the key was new (free, from
//!   the index read; `#[must_use]`, round 3 T1); `LookupMap::insert` returns `()` (no existence
//!   read). It requires `V: Leaf`, a value that owns no collections, so for a record that owns
//!   one plain `insert` does not compile (E0277 naming the fix). `try_insert(k, v) ->
//!   Result<&mut V, Occupied>` exists for every value type: it writes only if the key is absent
//!   and returns the bound value, and `Occupied` carries nothing (the stored value is not read).
//!   First-write-wins is `try_insert` or `entry(k).or_insert(v)`; `require!(m.insert(..), ..)`
//!   is the bug it replaces (the overwrite has already happened when the check fails).
//! - Enumeration order is the key vector's: insertion order until a remove moves the last key
//!   into the hole. `page(after, n)` / `page_with` / `keys_page` return `Err(CursorGone)` if the
//!   cursor key was removed since the last page (D8); a `#[handle_result]` view can return the
//!   `Result` as is. Offset paging, `iter().skip(from).take(n)` (where `skip` reads nothing),
//!   never fails but can skip a key after a removal behind the offset. An on-chain batch job
//!   must not page a map that changes between its calls. Every `Page` / `KeyPage` carries
//!   `revision` (T4), a counter in the map header that every removal bumps, read with the header
//!   at no extra cost: equal revisions across pages mean no removal in between. A map that
//!   empties keeps its header (`len` 0) so the revision never resets; the header goes with the
//!   record that owns the map.
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
#[cfg(any(test, kani))]
mod model;
/// Bolero / Kani harnesses for the seven guarantees and the named bug classes.
#[cfg(feature = "unstable")]
#[cfg(any(test, kani))]
mod verify;

#[cfg(feature = "unstable")]
pub use collections::{
    At, CursorGone, IndexIter, KeyPage, Lazy, LookupMap, LookupMapEntry, LookupSet, Map, MapEntry,
    Occupied, Page, Removal, Set, Vector, ViewIsReadOnly,
};
/// Deferred out of the first release (D3); hidden, kept compiling for the prototype.
#[cfg(feature = "unstable")]
#[doc(hidden)]
pub use collections::{Group, GroupMut, GroupedMap};
#[cfg(feature = "unstable")]
pub use key::{Path, StorageKey};
#[cfg(feature = "unstable")]
pub use migrating::{LegacyIterable, LegacyMap, MigrationTarget, Migrating};
#[cfg(feature = "unstable")]
pub use record::RecordError;
#[cfg(feature = "unstable")]
pub use root::{Root, __private, flush_root, load_root, raw_root, root_exists};
#[cfg(feature = "unstable")]
pub use value::{
    BorshLeaf, FieldDefault, Leaf, Record, Removable, UseGetForScalars, UseGetMutForScalars, Value,
    ViewField, ViewFieldMut,
};
/// `#[derive(Leaf)]`: opts a plain Borsh struct or enum into storage v2 as an opaque leaf
/// (implements [`BorshLeaf`], hence [`Value`], [`Leaf`] and [`Removable`]). T7.
#[cfg(feature = "unstable")]
pub use near_sdk_macros::Leaf;
