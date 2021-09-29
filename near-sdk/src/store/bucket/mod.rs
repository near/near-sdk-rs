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
    next_vacant: Option<BucketIndex>,
    occupied_count: u32,
    elements: Vector<Container<T>>,
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
        BorshSerialize::serialize(&self.next_vacant, writer)?;
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
            next_vacant: BorshDeserialize::deserialize(buf)?,
            occupied_count: BorshDeserialize::deserialize(buf)?,
            elements: BorshDeserialize::deserialize(buf)?,
        })
    }
}

#[derive(BorshDeserialize, BorshSerialize, Debug)]
enum Container<T> {
    /// Represents a filled cell of a value in the collection.
    Occupied(T),
    /// Representing that the cell has been removed, points to next empty cell, if one previously
    /// existed.
    Empty { next_index: Option<BucketIndex> },
}

impl<T> Container<T> {
    fn into_value(self) -> Option<T> {
        if let Container::Occupied(value) = self {
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
            .field("next_vacant", &self.next_vacant)
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
        Self { next_vacant: None, occupied_count: 0, elements: Vector::new(prefix) }
    }
    /// Returns length of values within the bucket.
    pub fn len(&self) -> u32 {
        self.occupied_count
    }
    /// Returns true if the bucket has no values.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl<T> Bucket<T>
where
    T: BorshSerialize + BorshDeserialize,
{
    /// Returns a reference to filled cell, if the value at the given index is valid. If the index
    /// is out of range or has been removed, returns `None`.
    pub fn get(&self, index: BucketIndex) -> Option<&T> {
        if let Container::Occupied(value) = self.elements.get(index.0)? {
            Some(value)
        } else {
            None
        }
    }
    /// Returns a mutable reference to filled cell, if the value at the given index is valid. If
    /// the index is out of range or has been removed, returns `None`.
    pub fn get_mut(&mut self, index: BucketIndex) -> Option<&mut T> {
        if let Container::Occupied(value) = self.elements.get_mut(index.0)? {
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
        let new_value = Container::Occupied(value);
        let inserted_index;
        if let Some(BucketIndex(vacant)) = self.next_vacant {
            // There is a vacant cell, put new value in that position
            let prev = self.elements.replace(vacant, new_value);
            inserted_index = vacant;

            if let Container::Empty { next_index } = prev {
                // Update pointer on bucket to this next index
                self.next_vacant = next_index;
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

        if matches!(entry, Container::Empty { .. }) {
            // Entry has already been cleared, return None
            return None;
        }

        // Take next pointer from bucket to attach to empty cell put in store
        let next_index = mem::take(&mut self.next_vacant);
        let prev = mem::replace(entry, Container::Empty { next_index });
        self.occupied_count -= 1;

        prev.into_value()
    }

    /// Flushes cached changes to storage. This retains any cached values in memory.
    pub fn flush(&mut self) {
        self.elements.flush()
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

    #[derive(Arbitrary, Debug)]
    enum Op {
        Insert(u8),
        Remove(u32),
        Flush,
        Reset,
        Get(u32),
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
                    }
                }
            }
        }
    }
}
