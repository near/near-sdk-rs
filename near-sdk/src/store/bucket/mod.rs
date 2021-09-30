mod iter;
pub use self::iter::{Iter, IterMut};

use super::{Vector, ERR_INCONSISTENT_STATE};
use crate::{env, IntoStorageKey};

use borsh::{BorshDeserialize, BorshSerialize};

use std::{fmt, mem};

/// Index for value within a bucket.
#[derive(BorshSerialize, BorshDeserialize, Debug, Hash, PartialEq, Eq, Clone, Copy)]
pub struct BucketIndex(u32);

/// Unordered container of values. This is similar to [`Vector`] except that values are not
/// re-arranged on removal, keeping the indices consistent. When an element is removed, it will
/// be replaced with an empty cell which will be populated on the next insertion.
pub struct Bucket<T>
where
    T: BorshSerialize,
{
    last_free: Option<BucketIndex>,
    occupied_count: u32,
    elements: Vector<Slot<T>>,
}

//? Manual implementations needed only because borsh derive is leaking field types
// https://github.com/near/borsh-rs/issues/41
impl<T> BorshSerialize for Bucket<T>
where
    T: BorshSerialize,
{
    fn serialize<W: borsh::maybestd::io::Write>(
        &self,
        writer: &mut W,
    ) -> Result<(), borsh::maybestd::io::Error> {
        BorshSerialize::serialize(&self.last_free, writer)?;
        BorshSerialize::serialize(&self.occupied_count, writer)?;
        BorshSerialize::serialize(&self.elements, writer)?;
        Ok(())
    }
}

impl<T> BorshDeserialize for Bucket<T>
where
    T: BorshSerialize,
{
    fn deserialize(buf: &mut &[u8]) -> Result<Self, borsh::maybestd::io::Error> {
        Ok(Self {
            last_free: BorshDeserialize::deserialize(buf)?,
            occupied_count: BorshDeserialize::deserialize(buf)?,
            elements: BorshDeserialize::deserialize(buf)?,
        })
    }
}

#[derive(BorshDeserialize, BorshSerialize, Debug)]
enum Slot<T> {
    /// Represents a filled cell of a value in the collection.
    Occupied(T),
    /// Representing that the cell has been removed, points to next empty cell, if one previously
    /// existed.
    Empty { next_free: Option<BucketIndex> },
}

impl<T> Slot<T> {
    fn into_value(self) -> Option<T> {
        if let Slot::Occupied(value) = self {
            Some(value)
        } else {
            None
        }
    }
}

impl<T> fmt::Debug for Bucket<T>
where
    T: BorshSerialize + BorshDeserialize + fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Bucket")
            .field("next_vacant", &self.last_free)
            .field("occupied_count", &self.occupied_count)
            .field("elements", &self.elements)
            .finish()
    }
}

impl<T> Extend<T> for Bucket<T>
where
    T: BorshSerialize + BorshDeserialize,
{
    fn extend<I>(&mut self, iter: I)
    where
        I: IntoIterator<Item = T>,
    {
        for item in iter {
            self.insert(item);
        }
    }
}

impl<T> Bucket<T>
where
    T: BorshSerialize,
{
    pub fn new<S: IntoStorageKey>(prefix: S) -> Self {
        Self { last_free: None, occupied_count: 0, elements: Vector::new(prefix) }
    }
    /// Returns length of values within the bucket.
    pub fn len(&self) -> u32 {
        self.occupied_count
    }
    /// Returns true if the bucket has no values.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Flushes cached changes to storage. This retains any cached values in memory.
    pub fn flush(&mut self) {
        self.elements.flush()
    }

    /// Clears the bucket, removing all values (including removed entries).
    pub fn clear(&mut self) {
        self.elements.clear();
        self.last_free = None;
        self.occupied_count = 0;
    }
}

impl<T> Bucket<T>
where
    T: BorshSerialize + BorshDeserialize,
{
    /// Returns a reference to filled cell, if the value at the given index is valid. If the index
    /// is out of range or has been removed, returns `None`.
    pub fn get(&self, index: BucketIndex) -> Option<&T> {
        if let Slot::Occupied(value) = self.elements.get(index.0)? {
            Some(value)
        } else {
            None
        }
    }
    /// Returns a mutable reference to filled cell, if the value at the given index is valid. If
    /// the index is out of range or has been removed, returns `None`.
    pub fn get_mut(&mut self, index: BucketIndex) -> Option<&mut T> {
        if let Slot::Occupied(value) = self.elements.get_mut(index.0)? {
            Some(value)
        } else {
            None
        }
    }
    /// Inserts new value into bucket. Returns the index that it was inserted at.
    ///
    /// # Panics
    ///
    /// Panics if new length exceeds `u32::MAX`
    pub fn insert(&mut self, value: T) -> BucketIndex {
        let new_value = Slot::Occupied(value);
        let inserted_index;
        if let Some(BucketIndex(vacant)) = self.last_free {
            // There is a vacant cell, put new value in that position
            let prev = self.elements.replace(vacant, new_value);
            inserted_index = vacant;

            if let Slot::Empty { next_free: next_index } = prev {
                // Update pointer on bucket to this next index
                self.last_free = next_index;
            } else {
                env::panic_str(ERR_INCONSISTENT_STATE)
            }
        } else {
            // No vacant cells, push and return index of pushed element
            self.elements.push(new_value);
            inserted_index = self.elements.len() - 1;
        }

        self.occupied_count += 1;
        BucketIndex(inserted_index)
    }

    /// Removes value at index in the bucket and returns the existing value, if any.
    pub fn remove(&mut self, index: BucketIndex) -> Option<T> {
        let entry = self.elements.get_mut(index.0)?;

        if matches!(entry, Slot::Empty { .. }) {
            // Entry has already been cleared, return None
            return None;
        }

        // Take next pointer from bucket to attach to empty cell put in store
        let next_index = mem::take(&mut self.last_free);
        let prev = mem::replace(entry, Slot::Empty { next_free: next_index });
        self.occupied_count -= 1;

        // Point next insert to this deleted index
        self.last_free = Some(index);

        prev.into_value()
    }

    /// Generates iterator for shared references to each value in the bucket.
    pub fn iter(&self) -> Iter<T> {
        Iter::new(self)
    }

    /// Generates iterator for exclusive references to each value in the bucket.
    pub fn iter_mut(&mut self) -> IterMut<T> {
        IterMut::new(self)
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use arbitrary::{Arbitrary, Unstructured};
    use rand::{RngCore, SeedableRng};

    use super::*;
    use crate::test_utils::test_env::setup_free;

    #[test]
    fn basic_functionality() {
        let mut bucket = Bucket::new(b"b");
        assert!(bucket.is_empty());
        let i5 = bucket.insert(5u8);
        let i3 = bucket.insert(3u8);
        assert_eq!(bucket.len(), 2);

        assert_eq!(bucket.get(i5), Some(&5));
        assert_eq!(bucket.remove(i5), Some(5));
        assert_eq!(bucket.len(), 1);

        *bucket.get_mut(i3).unwrap() = 4;
        assert_eq!(bucket.get(i3), Some(&4));
    }

    #[test]
    fn bucket_iterator() {
        let mut bucket = Bucket::new(b"b");

        bucket.insert(0u8);
        let rm = bucket.insert(1u8);
        bucket.insert(2u8);
        bucket.insert(3u8);
        bucket.remove(rm);
        let iter = bucket.iter();
        assert_eq!(iter.len(), 3);
        assert_eq!(iter.collect::<Vec<_>>(), [&0, &2, &3]);

        let iter = bucket.iter_mut().rev();
        assert_eq!(iter.collect::<Vec<_>>(), [&mut 3, &mut 2, &mut 0]);

        let mut iter = bucket.iter();
        assert_eq!(iter.nth(2), Some(&3));
        // Check fused iterator assumption that each following one will be None
        assert_eq!(iter.next(), None);
    }

    #[test]
    fn delete_internals() {
        let mut bucket = Bucket::new(b"b");
        let i0 = bucket.insert(0u8);
        let i1 = bucket.insert(1u8);
        let i2 = bucket.insert(2u8);
        let i3 = bucket.insert(3u8);

        // Remove 1 first
        bucket.remove(i1);
        assert_eq!(bucket.last_free, Some(i1));
        assert_eq!(bucket.occupied_count, 3);

        // Remove 0 next
        bucket.remove(i0);
        assert_eq!(bucket.last_free, Some(i0));
        assert_eq!(bucket.occupied_count, 2);

        // This should insert at index 0 (last deleted)
        let r5 = bucket.insert(5);
        assert_eq!(r5, i0);
        assert_eq!(bucket.last_free, Some(i1));
        assert_eq!(bucket.occupied_count, 3);

        bucket.remove(i3);
        bucket.remove(i2);
        assert_eq!(bucket.last_free, Some(i2));

        let r6 = bucket.insert(6);
        assert_eq!(r6, i2);

        let r7 = bucket.insert(7);
        assert_eq!(r7, i3);

        // Last spot to fill is index 1
        let r8 = bucket.insert(8);
        assert_eq!(r8, i1);
        assert!(bucket.last_free.is_none());
        assert_eq!(bucket.insert(9), BucketIndex(4));
    }

    #[derive(Arbitrary, Debug)]
    enum Op {
        Insert(u8),
        Remove(u32),
        Flush,
        Reset,
        Get(u32),
        Clear,
    }

    #[test]
    fn arbitrary() {
        setup_free();

        let mut rng = rand_xorshift::XorShiftRng::seed_from_u64(0);
        let mut buf = vec![0; 4096];
        for _ in 0..1024 {
            // Clear storage in-between runs
            crate::mock::with_mocked_blockchain(|b| b.take_storage());
            rng.fill_bytes(&mut buf);

            let mut sv = Bucket::new(b"v");
            let mut hm = HashMap::new();
            let u = Unstructured::new(&buf);
            if let Ok(ops) = Vec::<Op>::arbitrary_take_rest(u) {
                for op in ops {
                    match op {
                        Op::Insert(v) => {
                            let idx = sv.insert(v);
                            hm.insert(idx.0, v);
                            assert_eq!(sv.len() as usize, hm.len());
                        }
                        Op::Remove(i) => {
                            let i = i % (sv.len() + 1);
                            let r1 = sv.remove(BucketIndex(i));
                            let r2 = hm.remove(&i);
                            assert_eq!(r1, r2);
                            assert_eq!(sv.len() as usize, hm.len());
                        }
                        Op::Flush => {
                            sv.flush();
                        }
                        Op::Reset => {
                            let serialized = sv.try_to_vec().unwrap();
                            sv = Bucket::deserialize(&mut serialized.as_slice()).unwrap();
                        }
                        Op::Get(k) => {
                            let k = k % (sv.len() + 1);
                            let r1 = sv.get(BucketIndex(k));
                            let r2 = hm.get(&k);
                            assert_eq!(r1, r2)
                        }
                        Op::Clear => {
                            sv.clear();
                            hm.clear();
                        }
                    }
                }
            }
        }
    }
}