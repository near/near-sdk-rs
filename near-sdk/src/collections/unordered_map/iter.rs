use super::UnorderedMap;
use crate::collections::vector;
use borsh::{BorshDeserialize, BorshSerialize};
use std::iter::FusedIterator;

impl<'a, K, V> IntoIterator for &'a UnorderedMap<K, V>
where
    K: BorshSerialize + BorshDeserialize,
    V: BorshSerialize + BorshDeserialize,
{
    type Item = (K, V);
    type IntoIter = Iter<'a, K, V>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

/// An iterator over each element deserialized in the [`UnorderedMap`].
pub struct Iter<'a, K, V> {
    keys: vector::Iter<'a, K>,
    values: vector::Iter<'a, V>,
}

impl<'a, K, V> Iter<'a, K, V>
where
    K: BorshSerialize + BorshDeserialize,
    V: BorshSerialize + BorshDeserialize,
{
    pub(super) fn new(map: &'a UnorderedMap<K, V>) -> Self {
        Self { keys: map.keys.iter(), values: map.values.iter() }
    }
}

impl<'a, K, V> Iterator for Iter<'a, K, V>
where
    K: BorshSerialize + BorshDeserialize,
    V: BorshSerialize + BorshDeserialize,
{
    type Item = (K, V);

    fn next(&mut self) -> Option<Self::Item> {
        <Self as Iterator>::nth(self, 0)
    }

    fn nth(&mut self, n: usize) -> Option<Self::Item> {
        Some((self.keys.nth(n)?, self.values.nth(n)?))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.keys.size_hint()
    }

    fn count(self) -> usize {
        self.keys.count()
    }
}

impl<'a, K, V> ExactSizeIterator for Iter<'a, K, V>
where
    K: BorshSerialize + BorshDeserialize,
    V: BorshSerialize + BorshDeserialize,
{
}
impl<'a, K, V> FusedIterator for Iter<'a, K, V>
where
    K: BorshSerialize + BorshDeserialize,
    V: BorshSerialize + BorshDeserialize,
{
}

impl<'a, K, V> DoubleEndedIterator for Iter<'a, K, V>
where
    K: BorshSerialize + BorshDeserialize,
    V: BorshSerialize + BorshDeserialize,
{
    fn next_back(&mut self) -> Option<Self::Item> {
        <Self as DoubleEndedIterator>::nth_back(self, 0)
    }

    fn nth_back(&mut self, n: usize) -> Option<Self::Item> {
        Some((self.keys.nth_back(n)?, self.values.nth_back(n)?))
    }
}
