use super::Vector;
use borsh::{BorshDeserialize, BorshSerialize};
use std::iter::FusedIterator;

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
        debug_assert!(self.begin <= self.end);
        let n = n as u32;
        if self.begin + n >= self.end {
            return None;
        }
        let cur = self.begin + n;
        self.begin += 1 + n;
        self.vec.get(cur).expect("access is within bounds").into()
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
        debug_assert!(self.begin <= self.end);
        let n = n as u32;
        if self.begin >= self.end.saturating_sub(n) {
            return None;
        }
        self.end -= 1 + n;
        self.vec.get(self.end).expect("access is within bounds").into()
    }
}

/// An iterator over exclusive references to the elements of a storage vector.
#[cfg_attr(not(feature = "expensive-debug"), derive(Debug))]
pub struct IterMut<'a, T>
where
    T: BorshSerialize + BorshDeserialize,
{
    vec: &'a mut Vector<T>,
    begin: u32,
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
            // TODO double check if there is a better way around this lifetime issue
            //* SAFETY: The lifetime can be swapped here because we can assert that the iterator
            //*         will only give out one mutable reference for every individual item
            //*         during the iteration, and there is no overlap. This must be checked
            //*         that no element in this iterator is ever revisited.
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
        debug_assert!(self.begin <= self.end);
        let n = n as u32;
        if self.begin.saturating_add(n) >= self.end {
            return None;
        }
        let cur = self.begin + n;
        self.begin += 1 + n;
        self.get_mut(cur).expect("access is within bounds").into()
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
        debug_assert!(self.begin <= self.end);
        let n = n as u32;
        if self.begin >= self.end.saturating_sub(n) {
            return None;
        }
        self.end -= 1 + n;
        self.get_mut(self.end).expect("access is within bounds").into()
    }
}
