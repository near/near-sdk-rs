use std::iter::FusedIterator;

use borsh::{BorshDeserialize, BorshSerialize};

use super::{FreeList, Slot, ERR_INCONSISTENT_STATE};
use crate::{env, store::vec};

impl<'a, T> IntoIterator for &'a FreeList<T>
where
    T: BorshSerialize + BorshDeserialize,
{
    type Item = &'a T;
    type IntoIter = Iter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'a, T> IntoIterator for &'a mut FreeList<T>
where
    T: BorshSerialize + BorshDeserialize,
{
    type Item = &'a mut T;
    type IntoIter = IterMut<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}

fn decrement_count(count: &mut u32) {
    *count = count.checked_sub(1).unwrap_or_else(|| env::panic_str(ERR_INCONSISTENT_STATE));
}

/// An iterator over elements in the storage bucket. This only yields the occupied entries.
pub struct Iter<'a, T>
where
    T: BorshDeserialize + BorshSerialize,
{
    /// Values iterator which contains empty and filled cells.
    values: vec::Iter<'a, Slot<T>>,
    /// Amount of valid elements left to iterate.
    elements_left: u32,
}

impl<'a, T> Iter<'a, T>
where
    T: BorshDeserialize + BorshSerialize,
{
    pub(super) fn new(bucket: &'a FreeList<T>) -> Self {
        Self { values: bucket.elements.iter(), elements_left: bucket.occupied_count }
    }
}

impl<'a, T> Iterator for Iter<'a, T>
where
    T: BorshDeserialize + BorshSerialize,
{
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.elements_left == 0 {
            return None;
        }
        loop {
            match self.values.next() {
                Some(Slot::Empty { .. }) => continue,
                Some(Slot::Occupied(value)) => {
                    decrement_count(&mut self.elements_left);
                    return Some(value);
                }
                None => {
                    // This should never be hit, because if 0 occupied elements, should have
                    // returned before the loop
                    env::panic_str(ERR_INCONSISTENT_STATE)
                }
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let elements_left = self.elements_left as usize;
        (elements_left, Some(elements_left))
    }

    fn count(self) -> usize {
        self.elements_left as usize
    }
}

impl<'a, T> ExactSizeIterator for Iter<'a, T> where T: BorshSerialize + BorshDeserialize {}
impl<'a, T> FusedIterator for Iter<'a, T> where T: BorshSerialize + BorshDeserialize {}

impl<'a, T> DoubleEndedIterator for Iter<'a, T>
where
    T: BorshSerialize + BorshDeserialize,
{
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.elements_left == 0 {
            return None;
        }
        loop {
            match self.values.next_back() {
                Some(Slot::Empty { .. }) => continue,
                Some(Slot::Occupied(value)) => {
                    decrement_count(&mut self.elements_left);
                    return Some(value);
                }
                None => {
                    // This should never be hit, because if 0 occupied elements, should have
                    // returned before the loop
                    env::panic_str(ERR_INCONSISTENT_STATE)
                }
            }
        }
    }
}

/// An iterator over elements in the storage bucket. This only yields the occupied entries.
pub struct IterMut<'a, T>
where
    T: BorshDeserialize + BorshSerialize,
{
    /// Values iterator which contains empty and filled cells.
    values: vec::IterMut<'a, Slot<T>>,
    /// Amount of valid elements left to iterate.
    elements_left: u32,
}

impl<'a, T> IterMut<'a, T>
where
    T: BorshDeserialize + BorshSerialize,
{
    pub(super) fn new(bucket: &'a mut FreeList<T>) -> Self {
        Self { values: bucket.elements.iter_mut(), elements_left: bucket.occupied_count }
    }
}

impl<'a, T> Iterator for IterMut<'a, T>
where
    T: BorshDeserialize + BorshSerialize,
{
    type Item = &'a mut T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.elements_left == 0 {
            return None;
        }
        loop {
            match self.values.next() {
                Some(Slot::Empty { .. }) => continue,
                Some(Slot::Occupied(value)) => {
                    decrement_count(&mut self.elements_left);
                    return Some(value);
                }
                None => {
                    // This should never be hit, because if 0 occupied elements, should have
                    // returned before the loop
                    env::panic_str(ERR_INCONSISTENT_STATE)
                }
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let elements_left = self.elements_left as usize;
        (elements_left, Some(elements_left))
    }

    fn count(self) -> usize {
        self.elements_left as usize
    }
}

impl<'a, T> ExactSizeIterator for IterMut<'a, T> where T: BorshSerialize + BorshDeserialize {}
impl<'a, T> FusedIterator for IterMut<'a, T> where T: BorshSerialize + BorshDeserialize {}

impl<'a, T> DoubleEndedIterator for IterMut<'a, T>
where
    T: BorshSerialize + BorshDeserialize,
{
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.elements_left == 0 {
            return None;
        }
        loop {
            match self.values.next_back() {
                Some(Slot::Empty { .. }) => continue,
                Some(Slot::Occupied(value)) => {
                    decrement_count(&mut self.elements_left);
                    return Some(value);
                }
                None => {
                    // This should never be hit, because if 0 occupied elements, should have
                    // returned before the loop
                    env::panic_str(ERR_INCONSISTENT_STATE)
                }
            }
        }
    }
}
