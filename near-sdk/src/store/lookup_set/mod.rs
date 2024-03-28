mod impls;

use crate::store::key::{Identity, ToKey};
use crate::{env, IntoStorageKey};
use borsh::BorshSerialize;
use std::borrow::Borrow;
use std::fmt;
use std::marker::PhantomData;

use near_sdk_macros::near;

/// A non-iterable implementation of a set that stores its content directly on the storage trie.
///
/// This set stores the values under a hash of the set's `prefix` and [`BorshSerialize`] of the
/// value and transformed using the set's [`ToKey`] implementation.
///
/// The default hash function for [`LookupSet`] is [`Identity`] which just prefixes the serialized
/// key object and uses these bytes as the key. This is to be backwards-compatible with
/// [`collections::LookupSet`](crate::collections::LookupSet) and be fast for small keys.
/// To use a custom function, use [`with_hasher`]. Alternative builtin hash functions can be found
/// at [`near_sdk::store::key`](crate::store::key).
///
/// # Examples
/// ```
/// use near_sdk::store::LookupSet;
///
/// // Initializes a set, the generic types can be inferred to `LookupSet<String, Identity>`
/// // The `b"a"` parameter is a prefix for the storage keys of this data structure.
/// let mut books = LookupSet::new(b"a");
///
///
/// // Add some books.
/// books.insert("A Dance With Dragons".to_string());
/// books.insert("To Kill a Mockingbird".to_string());
/// books.insert("The Odyssey".to_string());
/// books.insert("The Great Gatsby".to_string());
///
/// // Check for a specific one.
/// assert!(!books.contains("The Winds of Winter"));
/// assert!(books.contains("The Odyssey"));
///
/// // Remove a book.
/// books.remove("The Odyssey");
///
/// assert!(!books.contains("The Odyssey"));
/// ```
///
/// [`with_hasher`]: Self::with_hasher
#[near(inside_nearsdk)]
pub struct LookupSet<T, H = Identity>
where
    T: BorshSerialize,
    H: ToKey,
{
    prefix: Box<[u8]>,

    #[borsh(skip)]
    hasher: PhantomData<fn() -> (T, H)>,
}

impl<T, H> fmt::Debug for LookupSet<T, H>
where
    T: BorshSerialize,
    H: ToKey,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LookupSet").field("prefix", &self.prefix).finish()
    }
}

impl<T> LookupSet<T, Identity>
where
    T: BorshSerialize,
{
    /// Initialize new [`LookupSet`] with the prefix provided.
    ///
    /// This prefix can be anything that implements [`IntoStorageKey`]. The prefix is used when
    /// storing and looking up values in storage to ensure no collisions with other collections.
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
    T: BorshSerialize,
    H: ToKey,
{
    /// Initialize a [`LookupSet`] with a custom hash function.
    ///
    /// # Example
    /// ```
    /// use near_sdk::store::{LookupSet, key::Keccak256};
    ///
    /// let map = LookupSet::<String, Keccak256>::with_hasher(b"m");
    /// ```
    pub fn with_hasher<S>(prefix: S) -> Self
    where
        S: IntoStorageKey,
    {
        Self { prefix: prefix.into_storage_key().into_boxed_slice(), hasher: Default::default() }
    }

    /// Returns `true` if the set contains the specified value.
    ///
    /// The value may be any borrowed form of the set's value type, but
    /// [`BorshSerialize`] on the borrowed form *must* match those for the value type.
    pub fn contains<Q: ?Sized>(&self, value: &Q) -> bool
    where
        T: Borrow<Q>,
        Q: BorshSerialize,
    {
        let lookup_key = H::to_key(&self.prefix, value, &mut Vec::new());
        env::storage_has_key(lookup_key.as_ref())
    }

    /// Adds a value to the set.
    ///
    /// If the set did not have this value present, true is returned.
    ///
    /// If the set did have this value present, false is returned.
    pub fn insert(&mut self, value: T) -> bool {
        let lookup_key = H::to_key(&self.prefix, &value, &mut Vec::new());
        !env::storage_write(lookup_key.as_ref(), &[])
    }

    /// Removes a value from the set. Returns whether the value was present in the set.
    ///
    /// The value may be any borrowed form of the set's value type, but
    /// [`BorshSerialize`] on the borrowed form *must* match those for the value type.
    pub fn remove<Q: ?Sized>(&mut self, value: &Q) -> bool
    where
        T: Borrow<Q>,
        Q: BorshSerialize,
    {
        let lookup_key = H::to_key(&self.prefix, value, &mut Vec::new());
        env::storage_remove(lookup_key.as_ref())
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[cfg(test)]
mod tests {
    use super::LookupSet;
    use crate::store::key::{Identity, Keccak256, ToKey};
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
    fn identity_compat_v1() {
        use crate::collections::LookupSet as LS1;

        let mut ls1 = LS1::new(b"m");
        ls1.insert(&8u8);
        ls1.insert(&0);
        assert!(ls1.contains(&8));

        let mut ls2 = LookupSet::<u8, _>::new(b"m");
        assert!(ls2.contains(&8u8));
        assert!(ls2.remove(&0));

        assert!(!ls1.contains(&0));
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
        let set = LookupSet::<u8>::new(b"m");

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
        set.insert(8u8);
        // Initialized value which state is `Inserted`
        assert!(set.contains(&8));
        set.remove(&8);
        // Initialized value which state is `Deleted`
        assert!(!set.contains(&8));
        set.insert(8);

        // Drop the set which should flush all data
        drop(set);

        let dup_set = LookupSet::<u8, _>::new(b"m");
        // Uninitialized value that is present on the trie
        assert!(dup_set.contains(&8));
        // Initialized value which state is `Present`
        assert!(dup_set.contains(&8));
    }

    #[test]
    fn test_insert_all_states() {
        let mut set = LookupSet::new(b"m");
        // Uninitialized value that is absent from the trie
        assert!(set.insert(8u8));
        // Initialized value which state is `Inserted`
        assert!(!set.insert(8));
        set.remove(&8);
        // Initialized value which state is `Deleted`
        assert!(set.insert(8));

        {
            let mut dup_set = LookupSet::new(b"m");
            // Uninitialized value that is present on the trie
            dup_set.insert(8u8);
            assert!(dup_set.contains(&8));
        }

        {
            let mut dup_set = LookupSet::new(b"m");
            assert!(dup_set.contains(&8));
            // Initialized value which state is `Present`
            dup_set.insert(8u8);
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
        set.insert(8u8);
        drop(set);

        {
            let mut dup_set = LookupSet::new(b"m");
            // Uninitialized value that is present on the trie
            assert!(dup_set.remove(&8));
            dup_set.insert(8u8);
        }

        {
            let mut dup_set = LookupSet::<u8, _>::new(b"m");
            assert!(dup_set.contains(&8));
            // Initialized value which state is `Present`
            assert!(dup_set.remove(&8));
        }
    }

    #[test]
    fn test_remove_present_after_insert() {
        let lookup_key = Identity::to_key(b"m", &8u8, &mut Vec::new());
        {
            // Scoped to make sure set is dropped and persist changes
            let mut set = LookupSet::new(b"m");
            set.insert(8u8);
        }
        assert!(crate::env::storage_has_key(&lookup_key));
        {
            let mut set = LookupSet::new(b"m");
            set.insert(8u8);
            set.remove(&8);
        }
        assert!(!crate::env::storage_has_key(&lookup_key));
        {
            let set = LookupSet::<u8, _>::new(b"m");
            assert!(!set.contains(&8u8));
        }
    }

    #[derive(Arbitrary, Debug)]
    enum Op {
        Insert(u8),
        Remove(u8),
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
