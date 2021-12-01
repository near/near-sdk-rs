mod impls;

use crate::crypto_hash::{CryptoHasher, Sha256};
use crate::store::LookupMap;
use crate::IntoStorageKey;
use borsh::{BorshDeserialize, BorshSerialize};
use std::borrow::Borrow;
use std::fmt;

#[derive(BorshSerialize, BorshDeserialize)]
pub struct LookupSet<T, H = Sha256>
where
    T: BorshSerialize + Ord,
    H: CryptoHasher<Digest = [u8; 32]>,
{
    map: LookupMap<T, (), H>,
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
        f.debug_struct("LookupSet").field("map", &self.map).finish()
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
        Self { map: LookupMap::with_hasher(prefix) }
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
        self.map.contains_key(value)
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
        self.map.insert(value, ()).is_none()
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
        self.map.remove(value).is_some()
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
        self.map.flush()
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

        assert_eq!(format!("{:?}", set), "LookupSet { map: LookupMap { prefix: [109] } }")
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

    #[derive(Arbitrary, Debug)]
    enum Op {
        Insert(u8),
        Remove(u8),
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
