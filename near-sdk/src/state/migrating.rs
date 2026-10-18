//! `Migrating<V1, V2>`: rekeying a v1 map into a v2 map one key at a time.
//!
//! Spec, "Coexistence and migration from v1", step 3. Reads try the new path and fall back to
//! the old; writes go to the new path and delete the old entry. For a v1 type that knows its
//! keys (`IterableMap`, `UnorderedMap`, `TreeMap`), `migrate_batch(n)` moves entries across
//! calls, and once `old_is_empty()` the field can be redeclared as the bare v2 map. A v1
//! `LookupMap` does not know its keys, so `migrate_batch` does not exist for it (a compile
//! error naming [`LegacyIterable`]) and the lazy path is permanent.
//!
//! Layout: the field is one tag of its record. The v1 half is stored inline in the record
//! (its Borsh bytes, i.e. its prefix), exactly as when the v1 map was a plain root field; the
//! v2 half is bound at `record ++ tag`, as a bare v2 field at that tag would be. So a
//! `token: FungibleToken`-style v1 root field can be replaced by `Migrating` at a *new* tag
//! (the v1 bytes move), and `Migrating` can later be replaced by the bare v2 map at the *same*
//! tag (the v2 entries are already where the bare map looks).
//!
//! Invariant (the one-side rule): every key is on at most one side. `insert` keeps it (it deletes
//! the old entry). It is not trusted blindly: `migrate_batch` never overwrites a key already on
//! the new side, and `remove` deletes from both sides, so a key on both sides (only reachable
//! through [`Migrating::new_side_mut_unchecked`] / [`Migrating::old_side_mut_unchecked`]) keeps
//! its new-side value and can be removed for good.

use super::collections::{LookupMap as NewLookupMap, Map as NewMap};
use super::key::{Path, StorageKey};
use super::record::RecordError;
use super::value::{Leaf, Removable, Value};
use borsh::{BorshDeserialize, BorshSerialize};

/// A v1 map that `Migrating` can read from and delete from.
pub trait LegacyMap {
    type K;
    type V;
    /// One read (or a cache hit for `store` types).
    fn legacy_get(&self, k: &Self::K) -> Option<Self::V>;
    /// Removes `k` if present, without returning the value where the type allows it.
    fn legacy_remove(&mut self, k: &Self::K);
    /// Every prefix this v1 map's raw storage keys start with (`prefix ++ ..`), checked by
    /// [`check_v1_prefix`]. Hashed keys are not listed (a 32-byte digest equals a v2 key only
    /// by a preimage). The default, empty, means unchecked; every v1 map in this crate
    /// overrides it.
    fn legacy_prefixes(&self) -> std::vec::Vec<std::vec::Vec<u8>> {
        std::vec::Vec::new()
    }
}

/// Why a v1 prefix can alias v2 keys (decision M4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum V1PrefixError {
    /// An empty prefix: the v1 key is the caller's borsh bytes, so a caller-chosen key can
    /// equal any v2 key (status-message's `LookupMap` has one).
    Empty,
    /// The prefix starts with the v2 root byte, so v1 keys share the v2 key space.
    RootByte(u8),
}

/// M4's migrate-time guard: a v1 map with key prefix `prefix` is disjoint from every v2 key
/// under root byte `root` if the prefix is non-empty and does not start with `root` (every v2
/// key starts with `root`). Conservative: a rejected prefix need not alias in practice.
/// Checked when a `Migrating` field is placed or loaded. A v1 field at the root that is not
/// wrapped in `Migrating` is not checked (the stated exception).
pub fn check_v1_prefix(prefix: &[u8], root: u8) -> Result<(), V1PrefixError> {
    match prefix.first() {
        None => Err(V1PrefixError::Empty),
        Some(&b) if b == root => Err(V1PrefixError::RootByte(root)),
        Some(_) => Ok(()),
    }
}

fn guard_v1_prefix<V1: LegacyMap>(old: &V1, path: &Path) {
    for p in old.legacy_prefixes() {
        if let Err(e) = check_v1_prefix(&p, path.as_bytes()[0]) {
            crate::env::panic_str(&v1_prefix_error(e));
        }
    }
}

fn v1_prefix_error(e: V1PrefixError) -> std::string::String {
    format!("Migrating: the v1 map's key prefix can alias v2 keys ({e:?}); give the v1 map a non-empty prefix whose first byte is not the v2 root byte")
}

/// A v1 map that can enumerate its keys, so `Migrating::migrate_batch` can drain it.
#[diagnostic::on_unimplemented(
    message = "`{Self}` cannot enumerate its keys, so `Migrating` cannot move its entries in batches",
    label = "`migrate_batch` / `old_is_empty` need a v1 map that knows its keys",
    note = "a v1 `LookupMap` stays on the lazy path permanently: reads fall back to it and each write moves one key; only an off-chain key list (an indexer) can drain it"
)]
pub trait LegacyIterable: LegacyMap {
    /// Removes up to `n` entries and returns them.
    fn legacy_take(&mut self, n: u32) -> std::vec::Vec<(Self::K, Self::V)>;
    fn legacy_is_empty(&self) -> bool;
}

/// The v2 side of a `Migrating`.
pub trait MigrationTarget: Value + Default {
    type K;
    type V;
    fn target_get(&self, k: &Self::K) -> Option<&Self::V>;
    fn target_insert(&mut self, k: Self::K, v: Self::V);
    /// Inserts only if `k` is absent; returns whether it inserted.
    fn target_try_insert(&mut self, k: Self::K, v: Self::V) -> bool;
    fn target_remove(&mut self, k: &Self::K);
}

impl<K: StorageKey + 'static, V: Removable + Leaf> MigrationTarget for NewLookupMap<K, V> {
    type K = K;
    type V = V;
    fn target_get(&self, k: &K) -> Option<&V> {
        self.get(k)
    }
    fn target_insert(&mut self, k: K, v: V) {
        self.insert(k, v)
    }
    /// 1 `storage_has_key` + 1 write (`insert` is 1 write).
    fn target_try_insert(&mut self, k: K, v: V) -> bool {
        self.try_insert(k, v).is_ok()
    }
    fn target_remove(&mut self, k: &K) {
        self.discard(k)
    }
}

impl<K: StorageKey + Clone + 'static, V: Removable + Leaf> MigrationTarget for NewMap<K, V> {
    type K = K;
    type V = V;
    fn target_get(&self, k: &K) -> Option<&V> {
        self.get(k)
    }
    fn target_insert(&mut self, k: K, v: V) {
        let _ = self.insert(k, v);
    }
    /// Same cost as `insert` for a new key: the index read it makes anyway decides.
    fn target_try_insert(&mut self, k: K, v: V) -> bool {
        self.try_insert(k, v).is_ok()
    }
    fn target_remove(&mut self, k: &K) {
        self.discard(k);
    }
}

/// A v1 map being rekeyed into a v2 map. See the module docs.
///
/// Must be declared `#[key(n, required)]` (the v1 half has no `Default`). Build it in a
/// `migrate` body with [`Migrating::new`] from the v1 map taken out of the old root.
pub struct Migrating<V1, V2> {
    old: V1,
    new: V2,
}

impl<V1, V2: Default> Migrating<V1, V2> {
    /// Wraps a v1 map. The v2 half is bound when the record is placed (or at the next call).
    pub fn new(old: V1) -> Self {
        Migrating { old, new: V2::default() }
    }
}

impl<V1, V2> Value for Migrating<V1, V2>
where
    V1: LegacyMap + BorshSerialize + BorshDeserialize + 'static,
    V2: MigrationTarget,
{
    // The v1 prefix is inline in the parent record; the v2 subtree hangs off the field path.
    const INLINE: bool = true;
    const MAX_SUBPATH: usize = V2::MAX_SUBPATH;

    fn encode_value(&self, _orig: Option<&[u8]>) -> std::vec::Vec<u8> {
        borsh::to_vec(&self.old).expect("borsh serialization cannot fail")
    }

    fn decode_value(bytes: &[u8], path: &Path) -> Result<Self, RecordError> {
        let old: V1 = borsh::from_slice(bytes).map_err(|e| RecordError::Leaf(e.to_string()))?;
        for p in old.legacy_prefixes() {
            check_v1_prefix(&p, path.as_bytes()[0]).map_err(|e| RecordError::Leaf(v1_prefix_error(e)))?;
        }
        let mut new = V2::default();
        new.bind(path);
        Ok(Migrating { old, new })
    }

    fn bind(&mut self, path: &Path) {
        guard_v1_prefix(&self.old, path);
        self.new.bind(path);
    }

    // The flush calls this on every root field before writing anything, so a `migrate` body
    // that installs an aliasing v1 map fails without a write.
    fn __write_pending(&self, path: &Path) {
        guard_v1_prefix(&self.old, path);
        self.new.__write_pending(path);
    }

    fn detach(&mut self) {
        self.new.detach();
    }
}

impl<V1, V2, K, V> Migrating<V1, V2>
where
    V1: LegacyMap<K = K, V = V>,
    V2: MigrationTarget<K = K, V = V>,
    K: Clone,
    V: Clone,
{
    /// New side first (1 read), then the old side on a miss (1 more read). A key on neither
    /// side costs both reads. Returns an owned value because the old side may not cache.
    pub fn get(&self, k: &K) -> Option<V> {
        match self.new.target_get(k) {
            Some(v) => Some(v.clone()),
            None => self.old.legacy_get(k),
        }
    }

    pub fn contains_key(&self, k: &K) -> bool {
        self.get(k).is_some()
    }

    /// Writes to the new side and deletes the old entry. If the key was already seen on the
    /// new side in this call (or reading it now finds it there), the old side is not touched,
    /// by the one-side invariant. Otherwise: 1 new-side read (a cache hit after `get`) +
    /// 1 old-side remove + the new-side write at flush.
    pub fn insert(&mut self, k: K, v: V) {
        let on_new = self.new.target_get(&k).is_some();
        if !on_new {
            self.old.legacy_remove(&k);
        }
        self.new.target_insert(k, v);
    }

    /// Removes `k` from both sides and returns the value a `get` would have returned (the new
    /// side's, if it has one). A key on the old side only costs what it did: the reads of `get`
    /// and 1 old-side remove. A key on the new side costs the new-side remove plus 1 old-side
    /// remove of a key it normally no longer holds (for a v1 `LookupMap` or `store::LookupMap`
    /// 1 `storage_remove`, for an `UnorderedMap` its index lookup, for a `TreeMap` its
    /// `contains_key` read: 1 read, no tree walk), so that an old value cannot resurrect through
    /// the fallback read even if the one-side rule was broken.
    pub fn remove(&mut self, k: &K) -> Option<V> {
        let v = self.get(k)?;
        if self.new.target_get(k).is_some() {
            self.new.target_remove(k);
        }
        self.old.legacy_remove(k);
        Some(v)
    }

    /// The v2 map, read-only: for reads `Migrating` does not forward (paging, `len`, `iter`
    /// of what has moved so far). A key that has not moved yet is not in it.
    pub fn new_side(&self) -> &V2 {
        &self.new
    }

    /// The v1 map, read-only.
    pub fn old_side(&self) -> &V1 {
        &self.old
    }

    /// The v2 map, mutable, **bypassing the one-side rule**: nothing deletes the old entry of a
    /// key written here, and nothing stops writing a key the old side still holds.
    ///
    /// Not needed by the migrate flow (`insert`, `remove` and `migrate_batch` cover it); it is
    /// for v2 operations `Migrating` does not forward, on keys known to be on the new side
    /// only. If the rule is broken anyway, the damage is bounded: a read prefers the new side,
    /// `migrate_batch` drops an old entry whose key is already on the new side instead of
    /// overwriting the newer value, and `remove` deletes both sides.
    pub fn new_side_mut_unchecked(&mut self) -> &mut V2 {
        &mut self.new
    }

    /// The v1 map, mutable, **bypassing the one-side rule**: a write here can put back a key
    /// that has already moved. See [`Migrating::new_side_mut_unchecked`] for what still holds.
    pub fn old_side_mut_unchecked(&mut self) -> &mut V1 {
        &mut self.old
    }

    /// Moves up to `n` entries from the old side to the new. Returns how many old entries it
    /// took (each is gone from the old side). An entry whose key is already on the new side is
    /// dropped, not moved: the new side is never older, so the old value never overwrites it.
    /// For a v2 `Map` target the skip is free (the index read `insert` makes anyway decides);
    /// for a v2 `LookupMap` target it costs 1 `storage_has_key` per entry.
    /// (The bound sits on the method, not the impl, so a v1 `LookupMap` gets the
    /// `LegacyIterable` diagnostic instead of a bare "method exists but bounds not satisfied".)
    pub fn migrate_batch(&mut self, n: u32) -> u32
    where
        V1: LegacyIterable,
    {
        let moved = self.old.legacy_take(n);
        let count = moved.len() as u32;
        for (k, v) in moved {
            self.new.target_try_insert(k, v);
        }
        count
    }

    /// When true, redeploy with the field declared as the bare v2 map at the same tag.
    pub fn old_is_empty(&self) -> bool
    where
        V1: LegacyIterable,
    {
        self.old.legacy_is_empty()
    }
}

// ---- v1 impls ----

#[cfg(feature = "legacy")]
mod legacy_collections {
    use super::*;
    use crate::collections::{LookupMap, TreeMap, UnorderedMap};

    impl<K: BorshSerialize, V: BorshSerialize + BorshDeserialize> LegacyMap for LookupMap<K, V> {
        type K = K;
        type V = V;
        fn legacy_get(&self, k: &K) -> Option<V> {
            self.get(k)
        }
        /// 1 `storage_remove` (the old value comes back in a register, not a read).
        fn legacy_remove(&mut self, k: &K) {
            self.remove_raw(&borsh::to_vec(k).expect("borsh"));
        }
        fn legacy_prefixes(&self) -> std::vec::Vec<std::vec::Vec<u8>> {
            vec![self.key_prefix().to_vec()]
        }
    }

    impl<K, V> LegacyMap for UnorderedMap<K, V>
    where
        K: BorshSerialize + BorshDeserialize,
        V: BorshSerialize + BorshDeserialize,
    {
        type K = K;
        type V = V;
        fn legacy_get(&self, k: &K) -> Option<V> {
            self.get(k)
        }
        fn legacy_remove(&mut self, k: &K) {
            self.remove(k);
        }
        fn legacy_prefixes(&self) -> std::vec::Vec<std::vec::Vec<u8>> {
            self.raw_prefixes()
        }
    }

    impl<K, V> LegacyIterable for UnorderedMap<K, V>
    where
        K: BorshSerialize + BorshDeserialize,
        V: BorshSerialize + BorshDeserialize,
    {
        fn legacy_take(&mut self, n: u32) -> std::vec::Vec<(K, V)> {
            let keys: std::vec::Vec<K> = self.keys().take(n as usize).collect();
            keys.into_iter().filter_map(|k| self.remove(&k).map(|v| (k, v))).collect()
        }
        fn legacy_is_empty(&self) -> bool {
            self.is_empty()
        }
    }

    impl<K, V> LegacyMap for TreeMap<K, V>
    where
        K: Ord + Clone + BorshSerialize + BorshDeserialize,
        V: BorshSerialize + BorshDeserialize,
    {
        type K = K;
        type V = V;
        fn legacy_get(&self, k: &K) -> Option<V> {
            self.get(k)
        }
        fn legacy_remove(&mut self, k: &K) {
            self.remove(k);
        }
        fn legacy_prefixes(&self) -> std::vec::Vec<std::vec::Vec<u8>> {
            self.raw_prefixes()
        }
    }

    impl<K, V> LegacyIterable for TreeMap<K, V>
    where
        K: Ord + Clone + BorshSerialize + BorshDeserialize,
        V: BorshSerialize + BorshDeserialize,
    {
        fn legacy_take(&mut self, n: u32) -> std::vec::Vec<(K, V)> {
            let keys: std::vec::Vec<K> = self.iter().take(n as usize).map(|(k, _)| k).collect();
            keys.into_iter().filter_map(|k| self.remove(&k).map(|v| (k, v))).collect()
        }
        fn legacy_is_empty(&self) -> bool {
            self.is_empty()
        }
    }
}

mod store_collections {
    use super::*;
    use crate::store::key::ToKey;
    use crate::store::{IterableMap, LookupMap};

    impl<K, V, H> LegacyMap for LookupMap<K, V, H>
    where
        K: BorshSerialize + Ord + Clone,
        V: BorshSerialize + BorshDeserialize + Clone,
        H: ToKey,
    {
        type K = K;
        type V = V;
        fn legacy_get(&self, k: &K) -> Option<V> {
            self.get(k).cloned()
        }
        /// No read: `set(k, None)` marks the entry for removal at the map's flush (its `Drop`,
        /// after the v2 flush).
        fn legacy_remove(&mut self, k: &K) {
            self.set(k.clone(), None);
        }
        fn legacy_prefixes(&self) -> std::vec::Vec<std::vec::Vec<u8>> {
            self.raw_prefix().map(<[u8]>::to_vec).into_iter().collect()
        }
    }

    impl<K, V, H> LegacyMap for IterableMap<K, V, H>
    where
        K: BorshSerialize + BorshDeserialize + Ord + Clone,
        V: BorshSerialize + BorshDeserialize + Clone,
        H: ToKey,
    {
        type K = K;
        type V = V;
        fn legacy_get(&self, k: &K) -> Option<V> {
            self.get(k).cloned()
        }
        fn legacy_remove(&mut self, k: &K) {
            self.remove(k);
        }
        fn legacy_prefixes(&self) -> std::vec::Vec<std::vec::Vec<u8>> {
            self.raw_prefixes()
        }
    }

    impl<K, V, H> LegacyIterable for IterableMap<K, V, H>
    where
        K: BorshSerialize + BorshDeserialize + Ord + Clone,
        V: BorshSerialize + BorshDeserialize + Clone,
        H: ToKey,
    {
        fn legacy_take(&mut self, n: u32) -> std::vec::Vec<(K, V)> {
            let keys: std::vec::Vec<K> = self.keys().take(n as usize).cloned().collect();
            keys.into_iter().filter_map(|k| self.remove(&k).map(|v| (k, v))).collect()
        }
        fn legacy_is_empty(&self) -> bool {
            self.is_empty()
        }
    }
}
