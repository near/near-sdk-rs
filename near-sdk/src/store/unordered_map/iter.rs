use std::iter::FusedIterator;

use borsh::{BorshDeserialize, BorshSerialize};

use super::{CryptoHasher, LookupMap, UnorderedMap, ValueAndIndex, ERR_INCONSISTENT_STATE};
use crate::{env, store::bucket};

impl<'a, K, V, H> IntoIterator for &'a UnorderedMap<K, V, H>
where
    K: BorshSerialize + Ord + BorshDeserialize + Clone,
    V: BorshSerialize + BorshDeserialize,
    H: CryptoHasher<Digest = [u8; 32]>,
{
    type Item = (&'a K, &'a V);
    type IntoIter = Iter<'a, K, V, H>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

// impl<'a, K, V, H> IntoIterator for &'a mut UnorderedMap<K, V, H>
// where
//     K: BorshSerialize + Ord + BorshDeserialize,
//     V: BorshSerialize,
//     H: CryptoHasher<Digest = [u8; 32]>,
// {
//     type Item = (&'a mut K, &'a mut V);
//     type IntoIter = IterMut<'a, K, V, H>;

//     fn into_iter(self) -> Self::IntoIter {
//         self.iter_mut()
//     }
// }

/// An iterator over elements in the storage bucket. This only yields the occupied entries.
pub struct Iter<'a, K, V, H>
where
    K: BorshSerialize + Ord + BorshDeserialize,
    V: BorshSerialize,
    H: CryptoHasher<Digest = [u8; 32]>,
{
    /// Values iterator which contains empty and filled cells.
    keys: bucket::Iter<'a, K>,
    /// Amount of valid elements left to iterate.
    values: &'a LookupMap<K, ValueAndIndex<V>, H>,
}

impl<'a, K, V, H> Iter<'a, K, V, H>
where
    K: BorshSerialize + Ord + BorshDeserialize,
    V: BorshSerialize,
    H: CryptoHasher<Digest = [u8; 32]>,
{
    pub(super) fn new(map: &'a UnorderedMap<K, V, H>) -> Self {
        Self { keys: map.keys.iter(), values: &map.values }
    }
}

impl<'a, K, V, H> Iterator for Iter<'a, K, V, H>
where
    K: BorshSerialize + Ord + BorshDeserialize + Clone,
    V: BorshSerialize + BorshDeserialize,
    H: CryptoHasher<Digest = [u8; 32]>,
{
    type Item = (&'a K, &'a V);

    fn next(&mut self) -> Option<Self::Item> {
        let key = self.keys.next()?;
        let entry = self.values.get(key).unwrap_or_else(|| env::panic_str(ERR_INCONSISTENT_STATE));

        Some((key, &entry.value))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.keys.size_hint()
    }

    fn count(self) -> usize {
        self.keys.count()
    }
}

impl<'a, K, V, H> ExactSizeIterator for Iter<'a, K, V, H>
where
    K: BorshSerialize + Ord + BorshDeserialize + Clone,
    V: BorshSerialize + BorshDeserialize,
    H: CryptoHasher<Digest = [u8; 32]>,
{
}
impl<'a, K, V, H> FusedIterator for Iter<'a, K, V, H>
where
    K: BorshSerialize + Ord + BorshDeserialize + Clone,
    V: BorshSerialize + BorshDeserialize,
    H: CryptoHasher<Digest = [u8; 32]>,
{
}

impl<'a, K, V, H> DoubleEndedIterator for Iter<'a, K, V, H>
where
    K: BorshSerialize + Ord + BorshDeserialize + Clone,
    V: BorshSerialize + BorshDeserialize,
    H: CryptoHasher<Digest = [u8; 32]>,
{
    fn next_back(&mut self) -> Option<Self::Item> {
        let key = self.keys.next_back()?;
        let entry = self.values.get(key).unwrap_or_else(|| env::panic_str(ERR_INCONSISTENT_STATE));

        Some((key, &entry.value))
    }
}

// /// An iterator over elements in the storage bucket. This only yields the occupied entries.
// pub struct IterMut<'a, K, V, H>
// where
//     K: BorshSerialize + Ord + BorshDeserialize,
//     V: BorshSerialize,
//     H: CryptoHasher<Digest = [u8; 32]>,
// {
//     /// Values iterator which contains empty and filled cells.
//     values: bucket::IterMut<'a, Container<K, V, H>>,
//     /// Amount of valid elements left to iterate.
//     elements_left: u32,
// }

// impl<'a, K, V, H> IterMut<'a, K, V, H>
// where
//     T: BorshDeserialize + BorshSerialize,
// {
//     pub(super) fn new(bucket: &'a mut UnorderedMap<K, V, H>) -> Self {
//         Self { values: bucket.elements.iter_mut(), elements_left: bucket.occupied_count }
//     }
//     fn decrement_elements(&mut self) {
//         self.elements_left = self
//             .elements_left
//             .checked_sub(1)
//             .unwrap_or_else(|| env::panic_str(ERR_INCONSISTENT_STATE));
//     }
// }

// impl<'a, K, V, H> Iterator for IterMut<'a, K, V, H>
// where
//     T: BorshDeserialize + BorshSerialize,
// {
//     type Item = &'a mut T;

//     fn next(&mut self) -> Option<Self::Item> {
//         if self.elements_left == 0 {
//             return None;
//         }
//         loop {
//             match self.values.next() {
//                 Some(Container::Empty { .. }) => continue,
//                 Some(Container::Occupied(value)) => {
//                     self.decrement_elements();
//                     return Some(value);
//                 }
//                 None => {
//                     // This should never be hit, because if 0 occupied elements, should have
//                     // returned before the loop
//                     env::panic_str(ERR_INCONSISTENT_STATE)
//                 }
//             }
//         }
//     }

//     fn size_hint(&self) -> (usize, Option<usize>) {
//         let elements_left = self.elements_left as usize;
//         (elements_left, Some(elements_left))
//     }

//     fn count(self) -> usize {
//         self.elements_left as usize
//     }
// }

// impl<'a, K, V, H> ExactSizeIterator for IterMut<'a, K, V, H> where
//     T: BorshSerialize + BorshDeserialize
// {
// }
// impl<'a, K, V, H> FusedIterator for IterMut<'a, K, V, H> where T: BorshSerialize + BorshDeserialize {}

// impl<'a, K, V, H> DoubleEndedIterator for IterMut<'a, K, V, H>
// where
//     T: BorshSerialize + BorshDeserialize,
// {
//     fn next_back(&mut self) -> Option<Self::Item> {
//         if self.elements_left == 0 {
//             return None;
//         }
//         loop {
//             match self.values.next_back() {
//                 Some(Container::Empty { .. }) => continue,
//                 Some(Container::Occupied(value)) => {
//                     self.decrement_elements();
//                     return Some(value);
//                 }
//                 None => {
//                     // This should never be hit, because if 0 occupied elements, should have
//                     // returned before the loop
//                     env::panic_str(ERR_INCONSISTENT_STATE)
//                 }
//             }
//         }
//     }
// }
