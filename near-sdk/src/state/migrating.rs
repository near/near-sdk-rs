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
//! Invariant: every key is on at most one side. Writes keep it (they delete the old entry);
//! so a key found on the new side needs no old-side delete.

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
    V1: BorshSerialize + BorshDeserialize + 'static,
    V2: MigrationTarget,
{
    // The v1 prefix is inline in the parent record; the v2 subtree hangs off the field path.
    const INLINE: bool = true;
    const MAX_SUBPATH: usize = V2::MAX_SUBPATH;

    fn encode_value(&self, _orig: Option<&[u8]>) -> std::vec::Vec<u8> {
        borsh::to_vec(&self.old).expect("borsh serialization cannot fail")
    }

    fn decode_value(bytes: &[u8], path: &Path) -> Result<Self, RecordError> {
        let old = borsh::from_slice(bytes).map_err(|e| RecordError::Leaf(e.to_string()))?;
        let mut new = V2::default();
        new.bind(path);
        Ok(Migrating { old, new })
    }

    fn bind(&mut self, path: &Path) {
        self.new.bind(path);
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

    /// Removes from whichever side holds `k`.
    pub fn remove(&mut self, k: &K) -> Option<V> {
        let v = self.get(k)?;
        if self.new.target_get(k).is_some() {
            self.new.target_remove(k);
        } else {
            self.old.legacy_remove(k);
        }
        Some(v)
    }

    /// The v2 map, for operations `Migrating` does not forward.
    pub fn new_side(&mut self) -> &mut V2 {
        &mut self.new
    }

    /// The v1 map.
    pub fn old_side(&mut self) -> &mut V1 {
        &mut self.old
    }

    /// Moves up to `n` entries from the old side to the new. Returns how many moved. By the
    /// one-side invariant an old entry is never stale, so it overwrites nothing newer.
    /// (The bound sits on the method, not the impl, so a v1 `LookupMap` gets the
    /// `LegacyIterable` diagnostic instead of a bare "method exists but bounds not satisfied".)
    pub fn migrate_batch(&mut self, n: u32) -> u32
    where
        V1: LegacyIterable,
    {
        let moved = self.old.legacy_take(n);
        let count = moved.len() as u32;
        for (k, v) in moved {
            self.new.target_insert(k, v);
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
