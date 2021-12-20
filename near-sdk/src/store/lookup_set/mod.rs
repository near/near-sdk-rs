mod impls;

use crate::crypto_hash::{CryptoHasher, Sha256};
use crate::{env, IntoStorageKey, StableMap};
use borsh::{BorshDeserialize, BorshSerialize};
use once_cell::unsync::OnceCell;
use std::borrow::Borrow;
use std::fmt;
use std::marker::PhantomData;

const ERR_ELEMENT_SERIALIZATION: &str = "Cannot serialize element";

type LookupKey = [u8; 32];

#[derive(BorshSerialize, BorshDeserialize)]
pub struct LookupSet<T, H = Sha256>
where
    T: BorshSerialize + Ord,
    H: CryptoHasher<Digest = [u8; 32]>,
{
    prefix: Box<[u8]>,

    /// Cache that keeps track the state of elements in the underlying set.
    #[borsh_skip]
    cache: StableMap<T, OnceCell<EntryState>>,

    #[borsh_skip]
    hasher: PhantomData<H>,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub(crate) enum EntryState {
    /// The element is cached as freshly inserted, but not necessarily absent from the trie
    Inserted,
    /// The element is cached as freshly deleted, but not necessarily present on the trie
    Deleted,
    /// The element is definitely present on the trie
    Present,
    /// The element is definitely absent from the trie
    Absent,
}

impl<T, H> Drop for LookupSet<T, H>
where
    T: BorshSerialize + Ord,
    H: CryptoHasher<Digest = [u8; 32]>,
{
    fn drop(&mut self) {
        self.flush()
    }
}

impl<T, H> fmt::Debug for LookupSet<T, H>
where
    T: BorshSerialize + Ord,
    H: CryptoHasher<Digest = [u8; 32]>,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LookupSet").field("prefix", &self.prefix).finish()
    }
}

impl<T> LookupSet<T, Sha256>
where
    T: BorshSerialize + Ord,
{
    #[inline]
    pub fn new<S>(prefix: S) -> Self
    where
        S: IntoStorageKey,
    {
        Self::with_hasher(prefix)
    }
}

impl<T, H> LookupSet<T, H>
where
    T: BorshSerialize + Ord,
    H: CryptoHasher<Digest = [u8; 32]>,
{
    fn lookup_key<Q: ?Sized>(prefix: &[u8], value: &Q, buffer: &mut Vec<u8>) -> LookupKey
    where
        Q: BorshSerialize,
        T: Borrow<Q>,
    {
        // Concat the prefix with serialized key and hash the bytes for the lookup key.
        buffer.extend(prefix);
        value.serialize(buffer).unwrap_or_else(|_| env::panic_str(ERR_ELEMENT_SERIALIZATION));

        H::hash(buffer)
    }

    fn contains_trie_element<Q: ?Sized>(prefix: &[u8], value: &Q) -> bool
    where
        Q: BorshSerialize,
        T: Borrow<Q>,
    {
        let lookup_key = Self::lookup_key(prefix, value, &mut Vec::new());
        env::storage_has_key(&lookup_key)
    }

    fn get_mut_inner<Q: ?Sized>(&mut self, value: &Q) -> &mut EntryState
    where
        T: Borrow<Q>,
        Q: BorshSerialize + ToOwned<Owned = T>,
    {
        let prefix = &self.prefix;
        //* ToOwned bound, which forces a clone, is required to be able to keep the value in the cache
        let entry = self.cache.get_mut(value.to_owned());
        entry.get_or_init(|| {
            if Self::contains_trie_element(prefix, value) {
                EntryState::Present
            } else {
                EntryState::Absent
            }
        });
        let entry = entry.get_mut().unwrap_or_else(|| env::abort());
        entry
    }

    /// Initialize a [`LookupSet`] with a custom hash function.
    ///
    /// # Example
    /// ```
    /// use near_sdk::crypto_hash::Keccak256;
    /// use near_sdk::store::LookupSet;
    ///
    /// let map = LookupSet::<String, Keccak256>::with_hasher(b"m");
    /// ```
    pub fn with_hasher<S>(prefix: S) -> Self
    where
        S: IntoStorageKey,
    {
        Self {
            prefix: prefix.into_storage_key().into_boxed_slice(),
            cache: Default::default(),
            hasher: Default::default(),
        }
    }

    /// Returns `true` if the set contains the specified value.
    ///
    /// The value may be any borrowed form of the set's value type, but
    /// [`BorshSerialize`], [`ToOwned<Owned = T>`](ToOwned) and [`Ord`] on the borrowed form *must*
    /// match those for the value type.
    pub fn contains<Q: ?Sized>(&self, value: &Q) -> bool
    where
        T: Borrow<Q>,
        Q: BorshSerialize + ToOwned<Owned = T> + Ord,
    {
        let entry_cell = self.cache.get(value.to_owned());
        match entry_cell.get_or_init(|| {
            let lookup_key = Self::lookup_key(&self.prefix, value, &mut Vec::new());
            let contains = env::storage_has_key(&lookup_key);
            if contains {
                EntryState::Present
            } else {
                EntryState::Absent
            }
        }) {
            EntryState::Inserted | EntryState::Present => true,
            EntryState::Deleted | EntryState::Absent => false,
        }
    }

    /// Adds a value to the set.
    ///
    /// If the set did not have this value present, true is returned.
    ///
    /// If the set did have this value present, false is returned.
    pub fn insert(&mut self, value: T) -> bool
    where
        T: Clone,
    {
        let entry = self.get_mut_inner(&value);
        match entry {
            EntryState::Inserted | EntryState::Present => false,
            EntryState::Deleted | EntryState::Absent => {
                *entry = EntryState::Inserted;
                true
            }
        }
    }

    /// Puts the given value into the set.
    ///
    /// This function will not return whether the passed value was already in the set.
    /// Use [`LookupSet::insert`] if you need that.
    pub fn put(&mut self, value: T) {
        let entry_cell = self.cache.get_mut(value);
        // It is safe to preemptively mark an entry as `Inserted` even if it is already present on
        // trie; it just means we will invoke `env::storage_write` one more time than strictly
        // necessary.
        entry_cell.get_or_init(|| EntryState::Inserted);
        let entry = entry_cell.get_mut().unwrap_or_else(|| env::abort());
        match entry {
            EntryState::Inserted | EntryState::Present => {}
            EntryState::Deleted | EntryState::Absent => *entry = EntryState::Inserted,
        }
    }

    /// Removes a value from the set. Returns whether the value was present in the set.
    ///
    /// The value may be any borrowed form of the set's value type, but
    /// [`BorshSerialize`], [`ToOwned<Owned = K>`](ToOwned) and [`Ord`] on the borrowed form *must*
    /// match those for the value type.
    pub fn remove<Q: ?Sized>(&mut self, value: &Q) -> bool
    where
        T: Borrow<Q>,
        Q: BorshSerialize + ToOwned<Owned = T> + Ord,
    {
        let entry = self.get_mut_inner(value);
        match entry {
            EntryState::Present | EntryState::Inserted => {
                *entry = EntryState::Deleted;
                true
            }
            EntryState::Deleted | EntryState::Absent => false,
        }
    }
}

impl<T, H> LookupSet<T, H>
where
    T: BorshSerialize + Ord,
    H: CryptoHasher<Digest = [u8; 32]>,
{
    /// Flushes the intermediate values of the set before this is called when the structure is
    /// [`Drop`]ed. This will write all modified values to storage but keep all cached values
    /// in memory.
    pub fn flush(&mut self) {
        let mut buf = Vec::new();
        for (k, v) in self.cache.inner().iter_mut() {
            if let Some(entry) = v.get_mut() {
                match entry {
                    EntryState::Inserted => {
                        buf.clear();
                        let lookup_key = Self::lookup_key(&self.prefix, k, &mut buf);
                        env::storage_write(&lookup_key, &[]);
                        *entry = EntryState::Present;
                    }
                    EntryState::Deleted => {
                        buf.clear();
                        let lookup_key = Self::lookup_key(&self.prefix, k, &mut buf);
                        env::storage_remove(&lookup_key);
                        *entry = EntryState::Absent;
                    }
                    EntryState::Present | EntryState::Absent => {}
                }
            }
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[cfg(test)]
mod tests {
    use super::LookupSet;
    use crate::crypto_hash::{Keccak256, Sha256};
    use crate::test_utils::test_env::setup_free;
    use arbitrary::{Arbitrary, Unstructured};
    use rand::seq::SliceRandom;
    use rand::RngCore;
    use rand::{Rng, SeedableRng};
    use std::collections::HashSet;

    #[test]
    fn test_insert_contains() {
        let mut set = LookupSet::new(b"m");
        let mut rng = rand_xorshift::XorShiftRng::seed_from_u64(0);
        let mut baseline = HashSet::new();
        for _ in 0..100 {
            let value = rng.gen::<u64>();
            set.insert(value);
            baseline.insert(value);
        }
        // Non existing
        for _ in 0..100 {
            let value = rng.gen::<u64>();
            assert_eq!(set.contains(&value), baseline.contains(&value));
        }
        // Existing
        for value in baseline.iter() {
            assert!(set.contains(value));
        }
    }

    #[test]
    fn test_insert_remove() {
        let mut set = LookupSet::new(b"m");
        let mut rng = rand_xorshift::XorShiftRng::seed_from_u64(1);
        let mut values = vec![];
        for _ in 0..100 {
            let value = rng.gen::<u64>();
            values.push(value);
            set.insert(value);
        }
        values.shuffle(&mut rng);
        for value in values {
            assert!(set.remove(&value));
        }
    }

    #[test]
    fn test_remove_last_reinsert() {
        let mut set = LookupSet::new(b"m");
        let value1 = 2u64;
        set.insert(value1);
        let value2 = 4u64;
        set.insert(value2);

        assert!(set.remove(&value2));
        assert!(set.insert(value2));
    }

    #[test]
    fn test_extend() {
        let mut set = LookupSet::new(b"m");
        let mut rng = rand_xorshift::XorShiftRng::seed_from_u64(4);
        let mut values = vec![];
        for _ in 0..100 {
            let value = rng.gen::<u64>();
            values.push(value);
            set.insert(value);
        }
        for _ in 0..10 {
            let mut tmp = vec![];
            for _ in 0..=(rng.gen::<u64>() % 20 + 1) {
                let value = rng.gen::<u64>();
                tmp.push(value);
            }
            values.extend(tmp.iter().cloned());
            set.extend(tmp.iter().cloned());
        }

        for value in values {
            assert!(set.contains(&value));
        }
    }

    #[test]
    fn test_debug() {
        let set = LookupSet::<u8, Sha256>::new(b"m");

        assert_eq!(format!("{:?}", set), "LookupSet { prefix: [109] }")
    }

    #[test]
    fn test_flush_on_drop() {
        let mut set = LookupSet::<_, Keccak256>::with_hasher(b"m");

        // Set a value, which does not write to storage yet
        set.insert(5u8);
        assert!(set.contains(&5u8));

        // Drop the set which should flush all data
        drop(set);

        // Create a duplicate set which references same data
        let dup_set = LookupSet::<u8, Keccak256>::with_hasher(b"m");

        // New map can now load the value
        assert!(dup_set.contains(&5u8));
    }

    #[test]
    fn test_contains_all_states() {
        let mut set = LookupSet::new(b"m");
        // Uninitialized value that is absent from the trie
        assert!(!set.contains(&8));
        // Initialized value which state is `Absent`
        assert!(!set.contains(&8));
        set.insert(8);
        // Initialized value which state is `Inserted`
        assert!(set.contains(&8));
        set.remove(&8);
        // Initialized value which state is `Deleted`
        assert!(!set.contains(&8));
        set.insert(8);

        // Drop the set which should flush all data
        drop(set);

        let dup_set = LookupSet::new(b"m");
        // Uninitialized value that is present on the trie
        assert!(dup_set.contains(&8));
        // Initialized value which state is `Present`
        assert!(dup_set.contains(&8));
    }

    #[test]
    fn test_insert_all_states() {
        let mut set = LookupSet::new(b"m");
        // Uninitialized value that is absent from the trie
        assert!(set.insert(8));
        // Initialized value which state is `Inserted`
        assert!(!set.insert(8));
        set.remove(&8);
        // Initialized value which state is `Deleted`
        assert!(set.insert(8));

        // Drop the set which should flush all data
        drop(set);

        let mut dup_set = LookupSet::new(b"m");
        // Uninitialized value that is present on the trie
        assert!(!dup_set.insert(8));
        // Initialized value which state is `Present`
        assert!(!dup_set.insert(8));
    }

    #[test]
    fn test_put_all_states() {
        let mut set = LookupSet::new(b"m");
        // Uninitialized value that is absent from the trie
        set.put(8);
        assert!(set.contains(&8));
        // Initialized value which state is `Inserted`
        set.put(8);
        assert!(set.contains(&8));

        set.remove(&8);
        // Initialized value which state is `Deleted`
        set.put(8);
        assert!(set.contains(&8));

        // Drop the set which should flush all data
        drop(set);

        {
            let mut dup_set = LookupSet::new(b"m");
            // Uninitialized value that is present on the trie
            dup_set.put(8);
            assert!(dup_set.contains(&8));
        }

        {
            let mut dup_set = LookupSet::new(b"m");
            assert!(dup_set.contains(&8));
            // Initialized value which state is `Present`
            dup_set.put(8);
            assert!(dup_set.contains(&8));
        }
    }

    #[test]
    fn test_remove_all_states() {
        let mut set = LookupSet::new(b"m");
        // Uninitialized value that is absent from the trie
        assert!(!set.remove(&8));
        // Initialized value which state is `Absent`
        assert!(!set.remove(&8));
        set.insert(8);
        // Initialized value which state is `Inserted`
        assert!(set.remove(&8));
        // Initialized value which state is `Deleted`
        assert!(!set.remove(&8));

        // Drop the set which should flush all data
        set.insert(8);
        drop(set);

        {
            let mut dup_set = LookupSet::new(b"m");
            // Uninitialized value that is present on the trie
            assert!(dup_set.remove(&8));
            dup_set.insert(8);
        }

        {
            let mut dup_set = LookupSet::new(b"m");
            assert!(dup_set.contains(&8));
            // Initialized value which state is `Present`
            assert!(dup_set.remove(&8));
        }
    }

    #[test]
    fn test_remove_present_after_put() {
        let lookup_key = LookupSet::<u8>::lookup_key(b"m", &8u8, &mut Vec::new());
        {
            // Scoped to make sure set is dropped and persist changes
            let mut set = LookupSet::new(b"m");
            set.put(8u8);
        }
        assert!(crate::env::storage_has_key(&lookup_key));
        {
            let mut set = LookupSet::new(b"m");
            set.put(8u8);
            set.remove(&8);
        }
        assert!(!crate::env::storage_has_key(&lookup_key));
        {
            let set = LookupSet::new(b"m");
            assert!(!set.contains(&8));
        }
    }

    #[derive(Arbitrary, Debug)]
    enum Op {
        Insert(u8),
        Remove(u8),
        Put(u8),
        Flush,
        Restore,
        Contains(u8),
    }

    #[test]
    fn test_arbitrary() {
        setup_free();

        let mut rng = rand_xorshift::XorShiftRng::seed_from_u64(0);
        let mut buf = vec![0; 4096];
        for _ in 0..512 {
            // Clear storage in-between runs
            crate::mock::with_mocked_blockchain(|b| b.take_storage());
            rng.fill_bytes(&mut buf);

            let mut ls = LookupSet::new(b"l");
            let mut hs = HashSet::new();
            let u = Unstructured::new(&buf);
            if let Ok(ops) = Vec::<Op>::arbitrary_take_rest(u) {
                for op in ops {
                    match op {
                        Op::Insert(v) => {
                            let r1 = ls.insert(v);
                            let r2 = hs.insert(v);
                            assert_eq!(r1, r2)
                        }
                        Op::Remove(v) => {
                            let r1 = ls.remove(&v);
                            let r2 = hs.remove(&v);
                            assert_eq!(r1, r2)
                        }
                        Op::Put(v) => {
                            ls.put(v);
                            hs.insert(v);

                            // Extra contains just to make sure put happened correctly
                            assert_eq!(ls.contains(&v), hs.contains(&v));
                        }
                        Op::Flush => {
                            ls.flush();
                        }
                        Op::Restore => {
                            ls = LookupSet::new(b"l");
                        }
                        Op::Contains(v) => {
                            let r1 = ls.contains(&v);
                            let r2 = hs.contains(&v);
                            assert_eq!(r1, r2)
                        }
                    }
                }
            }
        }
    }
}
