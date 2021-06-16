use borsh::{BorshDeserialize, BorshSerialize};
use std::{convert::TryInto, iter::FusedIterator};

use super::{Vector, ERR_INDEX_OUT_OF_BOUNDS};
use crate::env;

/// An interator over references to each element in the stored vector.
#[cfg_attr(not(feature = "expensive-debug"), derive(Debug))]
pub struct Iter<'a, T>
where
    T: BorshSerialize + BorshDeserialize,
{
    /// Underlying vector to iterate through
    vec: &'a Vector<T>,
    /// Initial index to start.
    begin: u32,
    /// End index to end interation.
    end: u32,
}

impl<'a, T> Iter<'a, T>
where
    T: BorshSerialize + BorshDeserialize,
{
    pub(super) fn new(vec: &'a Vector<T>) -> Self {
        Self { vec, begin: 0, end: vec.len() }
    }

    /// Returns number of elements left to iterate.
    fn remaining(&self) -> u32 {
        self.end - self.begin
    }
}

impl<'a, T> Iterator for Iter<'a, T>
where
    T: BorshSerialize + BorshDeserialize,
{
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        <Self as Iterator>::nth(self, 0)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.remaining() as usize;
        (remaining, Some(remaining))
    }

    fn count(self) -> usize {
        self.remaining() as usize
    }

    fn nth(&mut self, n: usize) -> Option<Self::Item> {
        let n: u32 = n.try_into().ok()?;
        self.begin = self.begin.saturating_add(n);
        if self.begin >= self.end {
            return None;
        }
        let cur = self.begin;
        self.begin += 1;
        self.vec.get(cur).unwrap_or_else(|| env::panic(ERR_INDEX_OUT_OF_BOUNDS)).into()
    }
}

impl<'a, T> ExactSizeIterator for Iter<'a, T> where T: BorshSerialize + BorshDeserialize {}
impl<'a, T> FusedIterator for Iter<'a, T> where T: BorshSerialize + BorshDeserialize {}

impl<'a, T> DoubleEndedIterator for Iter<'a, T>
where
    T: BorshSerialize + BorshDeserialize,
{
    fn next_back(&mut self) -> Option<Self::Item> {
        <Self as DoubleEndedIterator>::nth_back(self, 0)
    }

    fn nth_back(&mut self, n: usize) -> Option<Self::Item> {
        let n: u32 = n.try_into().ok()?;
        self.end = self.end.saturating_sub(n);
        if self.begin >= self.end {
            return None;
        }
        self.end -= 1;
        self.vec.get(self.end).unwrap_or_else(|| env::panic(ERR_INDEX_OUT_OF_BOUNDS)).into()
    }
}

/// An iterator over exclusive references to each element of a stored vector.
#[cfg_attr(not(feature = "expensive-debug"), derive(Debug))]
pub struct IterMut<'a, T>
where
    T: BorshSerialize + BorshDeserialize,
{
    /// Mutable reference to vector used to iterate through.
    vec: &'a mut Vector<T>,
    /// Start index of the remaining iterator.
    begin: u32,
    /// End index of the remaining iterator.
    end: u32,
}

impl<'a, T> IterMut<'a, T>
where
    T: BorshSerialize + BorshDeserialize,
{
    /// Creates a new iterator for the given storage vector.
    pub(crate) fn new(vec: &'a mut Vector<T>) -> Self {
        let len = vec.len();
        Self { vec, begin: 0, end: len }
    }

    /// Returns the amount of remaining elements to yield by the iterator.
    fn remaining(&self) -> u32 {
        self.end - self.begin
    }
}

impl<'a, T> IterMut<'a, T>
where
    T: BorshSerialize + BorshDeserialize,
{
    fn get_mut<'b>(&'b mut self, at: u32) -> Option<&'a mut T> {
        self.vec.get_mut(at).map(|value| {
            //* SAFETY: The lifetime can be swapped here because we can assert that the iterator
            //*         will only give out one mutable reference for every individual item
            //*         during the iteration, and there is no overlap. This must be checked
            //*         that no element in this iterator is ever revisited during iteration.
            unsafe { core::mem::transmute::<&'b mut T, &'a mut T>(value) }
        })
    }
}

impl<'a, T> Iterator for IterMut<'a, T>
where
    T: BorshSerialize + BorshDeserialize,
{
    type Item = &'a mut T;

    fn next(&mut self) -> Option<Self::Item> {
        <Self as Iterator>::nth(self, 0)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.remaining() as usize;
        (remaining, Some(remaining))
    }

    fn count(self) -> usize {
        self.remaining() as usize
    }

    fn nth(&mut self, n: usize) -> Option<Self::Item> {
        let n: u32 = n.try_into().ok()?;
        self.begin = self.begin.saturating_add(n);
        if self.begin >= self.end {
            return None;
        }
        let cur = self.begin;
        self.begin += 1;
        self.get_mut(cur).unwrap_or_else(|| env::panic(ERR_INDEX_OUT_OF_BOUNDS)).into()
    }
}

impl<'a, T> ExactSizeIterator for IterMut<'a, T> where T: BorshSerialize + BorshDeserialize {}
impl<'a, T> FusedIterator for IterMut<'a, T> where T: BorshSerialize + BorshDeserialize {}

impl<'a, T> DoubleEndedIterator for IterMut<'a, T>
where
    T: BorshSerialize + BorshDeserialize,
{
    fn next_back(&mut self) -> Option<Self::Item> {
        <Self as DoubleEndedIterator>::nth_back(self, 0)
    }

    fn nth_back(&mut self, n: usize) -> Option<Self::Item> {
        let n: u32 = n.try_into().ok()?;
        self.end = self.end.saturating_sub(n);
        if self.begin >= self.end {
            return None;
        }
        self.end -= 1;
        self.get_mut(self.end).unwrap_or_else(|| env::panic(ERR_INDEX_OUT_OF_BOUNDS)).into()
    }
}
