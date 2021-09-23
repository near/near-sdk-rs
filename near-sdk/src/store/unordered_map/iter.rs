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

impl<'a, K, V, H> IntoIterator for &'a mut UnorderedMap<K, V, H>
where
    K: BorshSerialize + Ord + BorshDeserialize + Clone,
    V: BorshSerialize + BorshDeserialize,
    H: CryptoHasher<Digest = [u8; 32]>,
{
    type Item = (&'a K, &'a mut V);
    type IntoIter = IterMut<'a, K, V, H>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}

/// An iterator over elements of a [`UnorderedMap`].
///
/// This `struct` is created by the `iter` method on [`UnorderedMap`]. See its
/// documentation for more.
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
        <Self as Iterator>::nth(self, 0)
    }

    fn nth(&mut self, n: usize) -> Option<Self::Item> {
        let key = self.keys.nth(n)?;
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
        <Self as DoubleEndedIterator>::nth_back(self, 0)
    }
    fn nth_back(&mut self, n: usize) -> Option<Self::Item> {
        let key = self.keys.nth_back(n)?;
        let entry = self.values.get(key).unwrap_or_else(|| env::panic_str(ERR_INCONSISTENT_STATE));

        Some((key, &entry.value))
    }
}

/// A mutable iterator over elements of a [`UnorderedMap`].
///
/// This `struct` is created by the `iter_mut` method on [`UnorderedMap`]. See its
/// documentation for more.
pub struct IterMut<'a, K, V, H>
where
    K: BorshSerialize + Ord + BorshDeserialize,
    V: BorshSerialize,
    H: CryptoHasher<Digest = [u8; 32]>,
{
    /// Values iterator which contains empty and filled cells.
    keys: bucket::IterMut<'a, K>,
    /// Amount of valid elements left to iterate.
    values: &'a mut LookupMap<K, ValueAndIndex<V>, H>,
}

impl<'a, K, V, H> IterMut<'a, K, V, H>
where
    K: BorshSerialize + Ord + BorshDeserialize,
    V: BorshSerialize,
    H: CryptoHasher<Digest = [u8; 32]>,
{
    pub(super) fn new(map: &'a mut UnorderedMap<K, V, H>) -> Self {
        Self { keys: map.keys.iter_mut(), values: &mut map.values }
    }
    fn get_entry_mut<'b>(&'b mut self, key: &'a K) -> (&'a K, &'a mut V)
    where
        K: Clone,
        V: BorshDeserialize,
    {
        let entry =
            self.values.get_mut(key).unwrap_or_else(|| env::panic_str(ERR_INCONSISTENT_STATE));
        //* SAFETY: The lifetime can be swapped here because we can assert that the iterator
        //*         will only give out one mutable reference for every individual key in the bucket
        //*         during the iteration, and there is no overlap. This operates under the
        //*         assumption that all elements in the bucket are unique and no hash collisions.
        //*         Because we use 32 byte hashes and all keys are verified unique based on the
        //*         `UnorderedMap` API, this is safe.
        let value = unsafe { &mut *(&mut entry.value as *mut V) };
        (key, value)
    }
}

impl<'a, K, V, H> Iterator for IterMut<'a, K, V, H>
where
    K: BorshSerialize + Ord + BorshDeserialize + Clone,
    V: BorshSerialize + BorshDeserialize,
    H: CryptoHasher<Digest = [u8; 32]>,
{
    type Item = (&'a K, &'a mut V);

    fn next(&mut self) -> Option<Self::Item> {
        <Self as Iterator>::nth(self, 0)
    }
    fn nth(&mut self, n: usize) -> Option<Self::Item> {
        let key = self.keys.nth(n)?;
        Some(self.get_entry_mut(key))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.keys.size_hint()
    }

    fn count(self) -> usize {
        self.keys.count()
    }
}

impl<'a, K, V, H> ExactSizeIterator for IterMut<'a, K, V, H>
where
    K: BorshSerialize + Ord + BorshDeserialize + Clone,
    V: BorshSerialize + BorshDeserialize,
    H: CryptoHasher<Digest = [u8; 32]>,
{
}
impl<'a, K, V, H> FusedIterator for IterMut<'a, K, V, H>
where
    K: BorshSerialize + Ord + BorshDeserialize + Clone,
    V: BorshSerialize + BorshDeserialize,
    H: CryptoHasher<Digest = [u8; 32]>,
{
}

impl<'a, K, V, H> DoubleEndedIterator for IterMut<'a, K, V, H>
where
    K: BorshSerialize + Ord + BorshDeserialize + Clone,
    V: BorshSerialize + BorshDeserialize,
    H: CryptoHasher<Digest = [u8; 32]>,
{
    fn next_back(&mut self) -> Option<Self::Item> {
        <Self as DoubleEndedIterator>::nth_back(self, 0)
    }
    fn nth_back(&mut self, n: usize) -> Option<Self::Item> {
        let key = self.keys.nth_back(n)?;
        Some(self.get_entry_mut(key))
    }
}

/// An iterator over the keys of a [`UnorderedMap`].
///
/// This `struct` is created by the `keys` method on [`UnorderedMap`]. See its
/// documentation for more.
pub struct Keys<'a, K: 'a>
where
    K: BorshSerialize + BorshDeserialize,
{
    inner: bucket::Iter<'a, K>,
}

impl<'a, K> Keys<'a, K>
where
    K: BorshSerialize + BorshDeserialize,
{
    pub(super) fn new<V, H>(map: &'a UnorderedMap<K, V, H>) -> Self
    where
        K: Ord,
        V: BorshSerialize,
        H: CryptoHasher<Digest = [u8; 32]>,
    {
        Self { inner: map.keys.iter() }
    }
}

impl<'a, K> Iterator for Keys<'a, K>
where
    K: BorshSerialize + BorshDeserialize,
{
    type Item = &'a K;

    fn next(&mut self) -> Option<&'a K> {
        self.inner.next()
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
    fn count(self) -> usize {
        self.inner.count()
    }
}

impl<'a, K> ExactSizeIterator for Keys<'a, K> where K: BorshSerialize + BorshDeserialize {}
impl<'a, K> FusedIterator for Keys<'a, K> where K: BorshSerialize + BorshDeserialize {}

impl<'a, K> DoubleEndedIterator for Keys<'a, K>
where
    K: BorshSerialize + Ord + BorshDeserialize,
{
    fn next_back(&mut self) -> Option<&'a K> {
        self.inner.next_back()
    }
}
