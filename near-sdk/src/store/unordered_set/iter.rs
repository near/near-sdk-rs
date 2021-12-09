use super::{CryptoHasher, UnorderedSet};
use crate::store::free_list::FreeListIndex;
use crate::store::{free_list, LookupMap};
use borsh::{BorshDeserialize, BorshSerialize};
use std::iter::{Chain, FusedIterator};
use std::marker::PhantomData;

impl<'a, T, H> IntoIterator for &'a UnorderedSet<T, H>
where
    T: BorshSerialize + Ord + BorshDeserialize + Clone,
    H: CryptoHasher<Digest = [u8; 32]>,
{
    type Item = &'a T;
    type IntoIter = Iter<'a, T, H>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

/// An iterator over elements of a [`UnorderedSet`].
///
/// This `struct` is created by the [`iter`] method on [`UnorderedSet`].
/// See its documentation for more.
///
/// [`iter`]: UnorderedSet::iter
pub struct Iter<'a, T, H>
where
    T: BorshSerialize + Ord + BorshDeserialize,
    H: CryptoHasher<Digest = [u8; 32]>,
{
    elements: free_list::Iter<'a, T>,

    el: PhantomData<H>,
}

impl<'a, T, H> Iter<'a, T, H>
where
    T: BorshSerialize + Ord + BorshDeserialize,
    H: CryptoHasher<Digest = [u8; 32]>,
{
    pub(super) fn new(set: &'a UnorderedSet<T, H>) -> Self {
        Self { elements: set.elements.iter(), el: Default::default() }
    }
}

impl<'a, T, H> Iterator for Iter<'a, T, H>
where
    T: BorshSerialize + Ord + BorshDeserialize + Clone,
    H: CryptoHasher<Digest = [u8; 32]>,
{
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        <Self as Iterator>::nth(self, 0)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.elements.size_hint()
    }

    fn count(self) -> usize {
        self.elements.count()
    }

    fn nth(&mut self, n: usize) -> Option<Self::Item> {
        self.elements.nth(n)
    }
}

impl<'a, T, H> ExactSizeIterator for Iter<'a, T, H>
where
    T: BorshSerialize + Ord + BorshDeserialize + Clone,
    H: CryptoHasher<Digest = [u8; 32]>,
{
}
impl<'a, T, H> FusedIterator for Iter<'a, T, H>
where
    T: BorshSerialize + Ord + BorshDeserialize + Clone,
    H: CryptoHasher<Digest = [u8; 32]>,
{
}

impl<'a, T, H> DoubleEndedIterator for Iter<'a, T, H>
where
    T: BorshSerialize + Ord + BorshDeserialize + Clone,
    H: CryptoHasher<Digest = [u8; 32]>,
{
    fn next_back(&mut self) -> Option<Self::Item> {
        <Self as DoubleEndedIterator>::nth_back(self, 0)
    }

    fn nth_back(&mut self, n: usize) -> Option<Self::Item> {
        self.elements.nth_back(n)
    }
}

/// A lazy iterator producing elements in the difference of `UnorderedSet`s.
///
/// This `struct` is created by the [`difference`] method on [`UnorderedSet`].
/// See its documentation for more.
///
/// [`difference`]: UnorderedSet::difference
pub struct Difference<'a, T, H>
where
    T: BorshSerialize + Ord + BorshDeserialize,
    H: CryptoHasher<Digest = [u8; 32]>,
{
    elements: free_list::Iter<'a, T>,

    other: &'a UnorderedSet<T, H>,

    el: PhantomData<H>,
}

impl<'a, T, H> Difference<'a, T, H>
where
    T: BorshSerialize + Ord + BorshDeserialize,
    H: CryptoHasher<Digest = [u8; 32]>,
{
    pub(super) fn new(set: &'a UnorderedSet<T, H>, other: &'a UnorderedSet<T, H>) -> Self {
        Self { elements: set.elements.iter(), other, el: Default::default() }
    }
}

impl<'a, T, H> Iterator for Difference<'a, T, H>
where
    T: BorshSerialize + Ord + BorshDeserialize + Clone,
    H: CryptoHasher<Digest = [u8; 32]>,
{
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        <Self as Iterator>::nth(self, 0)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (0, self.elements.size_hint().1)
    }

    fn count(mut self) -> usize {
        let mut count = 0usize;
        for element in self.elements.by_ref() {
            if !self.other.contains(element) {
                count += 1;
            }
        }
        count
    }

    fn nth(&mut self, n: usize) -> Option<Self::Item> {
        let mut count = 0usize;
        for element in self.elements.by_ref() {
            if !self.other.contains(element) {
                if count == n {
                    return Some(element);
                }
                count += 1;
            }
        }
        None
    }
}

impl<'a, T, H> FusedIterator for Difference<'a, T, H>
where
    T: BorshSerialize + Ord + BorshDeserialize + Clone,
    H: CryptoHasher<Digest = [u8; 32]>,
{
}

/// A lazy iterator producing elements in the intersection of `UnorderedSet`s.
///
/// This `struct` is created by the [`intersection`] method on [`UnorderedSet`].
/// See its documentation for more.
///
/// [`intersection`]: UnorderedSet::intersection
pub struct Intersection<'a, T, H>
where
    T: BorshSerialize + Ord + BorshDeserialize,
    H: CryptoHasher<Digest = [u8; 32]>,
{
    elements: free_list::Iter<'a, T>,

    other: &'a UnorderedSet<T, H>,

    el: PhantomData<H>,
}

impl<'a, T, H> Intersection<'a, T, H>
where
    T: BorshSerialize + Ord + BorshDeserialize,
    H: CryptoHasher<Digest = [u8; 32]>,
{
    pub(super) fn new(set: &'a UnorderedSet<T, H>, other: &'a UnorderedSet<T, H>) -> Self {
        Self { elements: set.elements.iter(), other, el: Default::default() }
    }
}

impl<'a, T, H> Iterator for Intersection<'a, T, H>
where
    T: BorshSerialize + Ord + BorshDeserialize + Clone,
    H: CryptoHasher<Digest = [u8; 32]>,
{
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        <Self as Iterator>::nth(self, 0)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (0, self.elements.size_hint().1)
    }

    fn count(mut self) -> usize {
        let mut count = 0usize;
        for element in self.elements.by_ref() {
            if self.other.contains(element) {
                count += 1;
            }
        }
        count
    }

    fn nth(&mut self, n: usize) -> Option<Self::Item> {
        let mut count = 0usize;
        for element in self.elements.by_ref() {
            if self.other.contains(element) {
                if count == n {
                    return Some(element);
                }
                count += 1;
            }
        }
        None
    }
}

impl<'a, T, H> FusedIterator for Intersection<'a, T, H>
where
    T: BorshSerialize + Ord + BorshDeserialize + Clone,
    H: CryptoHasher<Digest = [u8; 32]>,
{
}

/// A lazy iterator producing elements in the symmetrical difference of `UnorderedSet`s.
///
/// This `struct` is created by the [`symmetrical_difference`] method on [`UnorderedSet`].
/// See its documentation for more.
///
/// [`symmetrical_difference`]: UnorderedSet::symmetrical_difference
pub struct SymmetricDifference<'a, T, H>
where
    T: BorshSerialize + Ord + BorshDeserialize,
    H: CryptoHasher<Digest = [u8; 32]>,
{
    iter: Chain<Difference<'a, T, H>, Difference<'a, T, H>>,
}

impl<'a, T, H> SymmetricDifference<'a, T, H>
where
    T: BorshSerialize + Ord + BorshDeserialize + Clone,
    H: CryptoHasher<Digest = [u8; 32]>,
{
    pub(super) fn new(set: &'a UnorderedSet<T, H>, other: &'a UnorderedSet<T, H>) -> Self {
        Self { iter: set.difference(other).chain(other.difference(set)) }
    }
}

impl<'a, T, H> Iterator for SymmetricDifference<'a, T, H>
where
    T: BorshSerialize + Ord + BorshDeserialize + Clone,
    H: CryptoHasher<Digest = [u8; 32]>,
{
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}

impl<'a, T, H> FusedIterator for SymmetricDifference<'a, T, H>
where
    T: BorshSerialize + Ord + BorshDeserialize + Clone,
    H: CryptoHasher<Digest = [u8; 32]>,
{
}

/// A lazy iterator producing elements in the union of `UnorderedSet`s.
///
/// This `struct` is created by the [`union`] method on [`UnorderedSet`].
/// See its documentation for more.
///
/// [`union`]: UnorderedSet::union
pub struct Union<'a, T, H>
where
    T: BorshSerialize + Ord + BorshDeserialize,
    H: CryptoHasher<Digest = [u8; 32]>,
{
    iter: Chain<Iter<'a, T, H>, Difference<'a, T, H>>,
}

impl<'a, T, H> Union<'a, T, H>
where
    T: BorshSerialize + Ord + BorshDeserialize + Clone,
    H: CryptoHasher<Digest = [u8; 32]>,
{
    pub(super) fn new(set: &'a UnorderedSet<T, H>, other: &'a UnorderedSet<T, H>) -> Self {
        Self { iter: set.iter().chain(other.difference(set)) }
    }
}

impl<'a, T, H> Iterator for Union<'a, T, H>
where
    T: BorshSerialize + Ord + BorshDeserialize + Clone,
    H: CryptoHasher<Digest = [u8; 32]>,
{
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}

impl<'a, T, H> FusedIterator for Union<'a, T, H>
where
    T: BorshSerialize + Ord + BorshDeserialize + Clone,
    H: CryptoHasher<Digest = [u8; 32]>,
{
}

/// A draining iterator for [`UnorderedMap<K, V, H>`].
///
/// This `struct` is created by the [`drain`] method on [`UnorderedSet`].
/// See its documentation for more.
///
/// [`drain`]: UnorderedSet::drain
#[derive(Debug)]
pub struct Drain<'a, T, H>
where
    T: BorshSerialize + BorshDeserialize + Ord,
    H: CryptoHasher<Digest = [u8; 32]>,
{
    elements: free_list::Drain<'a, T>,

    index: &'a mut LookupMap<T, FreeListIndex, H>,

    el: PhantomData<H>,
}

impl<'a, T, H> Drain<'a, T, H>
where
    T: BorshSerialize + BorshDeserialize + Ord,
    H: CryptoHasher<Digest = [u8; 32]>,
{
    pub(crate) fn new(set: &'a mut UnorderedSet<T, H>) -> Self {
        Self { elements: set.elements.drain(), index: &mut set.index, el: Default::default() }
    }

    fn remaining(&self) -> usize {
        self.elements.remaining()
    }
}

impl<'a, T, H> Iterator for Drain<'a, T, H>
where
    T: BorshSerialize + BorshDeserialize + Ord + Clone,
    H: CryptoHasher<Digest = [u8; 32]>,
{
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        let key = self.elements.next()?;
        self.index.remove(&key);
        Some(key)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.remaining();
        (remaining, Some(remaining))
    }

    fn count(self) -> usize {
        self.remaining()
    }
}

impl<'a, T, H> ExactSizeIterator for Drain<'a, T, H>
where
    T: BorshSerialize + Ord + BorshDeserialize + Clone,
    H: CryptoHasher<Digest = [u8; 32]>,
{
}

impl<'a, T, H> FusedIterator for Drain<'a, T, H>
where
    T: BorshSerialize + Ord + BorshDeserialize + Clone,
    H: CryptoHasher<Digest = [u8; 32]>,
{
}

impl<'a, T, H> DoubleEndedIterator for Drain<'a, T, H>
where
    T: BorshSerialize + Ord + BorshDeserialize + Clone,
    H: CryptoHasher<Digest = [u8; 32]>,
{
    fn next_back(&mut self) -> Option<Self::Item> {
        self.elements.next_back()
    }
}
