use super::IterableSet;
use crate::store::iterable_set::VecIndex;
use crate::store::key::ToKey;
use crate::store::{vec, LookupMap};
use borsh::{BorshDeserialize, BorshSerialize};
use std::iter::{Chain, FusedIterator};

impl<'a, T, H> IntoIterator for &'a IterableSet<T, H>
where
    T: BorshSerialize + Ord + BorshDeserialize + Clone,
    H: ToKey,
{
    type Item = &'a T;
    type IntoIter = Iter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

/// An iterator over elements of a [`IterableSet`].
///
/// This `struct` is created by the [`iter`] method on [`IterableSet`].
/// See its documentation for more.
///
/// [`iter`]: IterableSet::iter
#[derive(Clone)]
pub struct Iter<'a, T>
where
    T: BorshSerialize + Ord + BorshDeserialize,
{
    elements: vec::Iter<'a, T>,
}

impl<'a, T> Iter<'a, T>
where
    T: BorshSerialize + Ord + BorshDeserialize,
{
    pub(super) fn new<H>(set: &'a IterableSet<T, H>) -> Self
    where
        H: ToKey,
    {
        Self { elements: set.elements.iter() }
    }
}

impl<'a, T> Iterator for Iter<'a, T>
where
    T: BorshSerialize + Ord + BorshDeserialize,
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

impl<'a, T> ExactSizeIterator for Iter<'a, T> where T: BorshSerialize + Ord + BorshDeserialize {}
impl<'a, T> FusedIterator for Iter<'a, T> where T: BorshSerialize + Ord + BorshDeserialize {}

impl<'a, T> DoubleEndedIterator for Iter<'a, T>
where
    T: BorshSerialize + Ord + BorshDeserialize,
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
/// This `struct` is created by the [`difference`] method on [`IterableSet`].
/// See its documentation for more.
///
/// [`difference`]: IterableSet::difference
pub struct Difference<'a, T, H>
where
    T: BorshSerialize + Ord + BorshDeserialize,
    H: ToKey,
{
    elements: vec::Iter<'a, T>,

    other: &'a IterableSet<T, H>,
}

impl<'a, T, H> Difference<'a, T, H>
where
    T: BorshSerialize + Ord + BorshDeserialize,
    H: ToKey,
{
    pub(super) fn new(set: &'a IterableSet<T, H>, other: &'a IterableSet<T, H>) -> Self {
        Self { elements: set.elements.iter(), other }
    }
}

impl<'a, T, H> Iterator for Difference<'a, T, H>
where
    T: BorshSerialize + Ord + BorshDeserialize + Clone,
    H: ToKey,
{
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let elt = self.elements.next()?;
            if !self.other.contains(elt) {
                return Some(elt);
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (0, self.elements.size_hint().1)
    }
}

impl<'a, T, H> FusedIterator for Difference<'a, T, H>
where
    T: BorshSerialize + Ord + BorshDeserialize + Clone,
    H: ToKey,
{
}

/// A lazy iterator producing elements in the intersection of `UnorderedSet`s.
///
/// This `struct` is created by the [`intersection`] method on [`IterableSet`].
/// See its documentation for more.
///
/// [`intersection`]: IterableSet::intersection
pub struct Intersection<'a, T, H>
where
    T: BorshSerialize + Ord + BorshDeserialize,
    H: ToKey,
{
    elements: vec::Iter<'a, T>,

    other: &'a IterableSet<T, H>,
}

impl<'a, T, H> Intersection<'a, T, H>
where
    T: BorshSerialize + Ord + BorshDeserialize,
    H: ToKey,
{
    pub(super) fn new(set: &'a IterableSet<T, H>, other: &'a IterableSet<T, H>) -> Self {
        Self { elements: set.elements.iter(), other }
    }
}

impl<'a, T, H> Iterator for Intersection<'a, T, H>
where
    T: BorshSerialize + Ord + BorshDeserialize + Clone,
    H: ToKey,
{
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let elt = self.elements.next()?;
            if self.other.contains(elt) {
                return Some(elt);
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (0, self.elements.size_hint().1)
    }
}

impl<'a, T, H> FusedIterator for Intersection<'a, T, H>
where
    T: BorshSerialize + Ord + BorshDeserialize + Clone,
    H: ToKey,
{
}

/// A lazy iterator producing elements in the symmetrical difference of [`IterableSet`]s.
///
/// This `struct` is created by the [`symmetric_difference`] method on [`IterableSet`].
/// See its documentation for more.
///
/// [`symmetric_difference`]: IterableSet::symmetric_difference
pub struct SymmetricDifference<'a, T, H>
where
    T: BorshSerialize + Ord + BorshDeserialize,
    H: ToKey,
{
    iter: Chain<Difference<'a, T, H>, Difference<'a, T, H>>,
}

impl<'a, T, H> SymmetricDifference<'a, T, H>
where
    T: BorshSerialize + Ord + BorshDeserialize + Clone,
    H: ToKey,
{
    pub(super) fn new(set: &'a IterableSet<T, H>, other: &'a IterableSet<T, H>) -> Self {
        Self { iter: set.difference(other).chain(other.difference(set)) }
    }
}

impl<'a, T, H> Iterator for SymmetricDifference<'a, T, H>
where
    T: BorshSerialize + Ord + BorshDeserialize + Clone,
    H: ToKey,
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
    H: ToKey,
{
}

/// A lazy iterator producing elements in the union of `UnorderedSet`s.
///
/// This `struct` is created by the [`union`] method on [`IterableSet`].
/// See its documentation for more.
///
/// [`union`]: IterableSet::union
pub struct Union<'a, T, H>
where
    T: BorshSerialize + Ord + BorshDeserialize,
    H: ToKey,
{
    iter: Chain<Iter<'a, T>, Difference<'a, T, H>>,
}

impl<'a, T, H> Union<'a, T, H>
where
    T: BorshSerialize + Ord + BorshDeserialize + Clone,
    H: ToKey,
{
    pub(super) fn new(set: &'a IterableSet<T, H>, other: &'a IterableSet<T, H>) -> Self {
        Self { iter: set.iter().chain(other.difference(set)) }
    }
}

impl<'a, T, H> Iterator for Union<'a, T, H>
where
    T: BorshSerialize + Ord + BorshDeserialize + Clone,
    H: ToKey,
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
    H: ToKey,
{
}

/// A draining iterator for [`IterableSet`].
///
/// This `struct` is created by the [`drain`] method on [`IterableSet`].
/// See its documentation for more.
///
/// [`drain`]: IterableSet::drain
#[derive(Debug)]
pub struct Drain<'a, T, H>
where
    T: BorshSerialize + BorshDeserialize + Ord,
    H: ToKey,
{
    elements: vec::Drain<'a, T>,

    index: &'a mut LookupMap<T, VecIndex, H>,
}

impl<'a, T, H> Drain<'a, T, H>
where
    T: BorshSerialize + BorshDeserialize + Ord,
    H: ToKey,
{
    pub(crate) fn new(set: &'a mut IterableSet<T, H>) -> Self {
        Self { elements: set.elements.drain(..), index: &mut set.index }
    }

    fn remaining(&self) -> usize {
        self.elements.remaining()
    }
}

impl<'a, T, H> Iterator for Drain<'a, T, H>
where
    T: BorshSerialize + BorshDeserialize + Ord + Clone,
    H: ToKey,
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
    H: ToKey,
{
}

impl<'a, T, H> FusedIterator for Drain<'a, T, H>
where
    T: BorshSerialize + Ord + BorshDeserialize + Clone,
    H: ToKey,
{
}

impl<'a, T, H> DoubleEndedIterator for Drain<'a, T, H>
where
    T: BorshSerialize + Ord + BorshDeserialize + Clone,
    H: ToKey,
{
    fn next_back(&mut self) -> Option<Self::Item> {
        self.elements.next_back()
    }
}
