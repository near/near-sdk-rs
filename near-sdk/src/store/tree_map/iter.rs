use std::ops::Bound;
use std::vec::Vec;
use std::{borrow::Borrow, iter::FusedIterator};

use borsh::{BorshDeserialize, BorshSerialize};

use super::{expect, LookupMap, Tree, TreeMap};
use crate::store::free_list::FreeListIndex;
use crate::store::key::ToKey;

impl<'a, K, V, H> IntoIterator for &'a TreeMap<K, V, H>
where
    K: BorshSerialize + Ord + BorshDeserialize + Clone,
    V: BorshSerialize + BorshDeserialize,
    H: ToKey,
{
    type Item = (&'a K, &'a V);
    type IntoIter = Iter<'a, K, V, H>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'a, K, V, H> IntoIterator for &'a mut TreeMap<K, V, H>
where
    K: BorshSerialize + Ord + BorshDeserialize + Clone,
    V: BorshSerialize + BorshDeserialize,
    H: ToKey,
{
    type Item = (&'a K, &'a mut V);
    type IntoIter = IterMut<'a, K, V, H>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}

/// An iterator over elements of a [`TreeMap`], in sorted order.
///
/// This `struct` is created by the `iter` method on [`TreeMap`].
pub struct Iter<'a, K, V, H>
where
    K: BorshSerialize + Ord + BorshDeserialize,
    V: BorshSerialize,
    H: ToKey,
{
    keys: Keys<'a, K>,
    values: &'a LookupMap<K, V, H>,
}

impl<'a, K, V, H> Iter<'a, K, V, H>
where
    K: BorshSerialize + Ord + BorshDeserialize,
    V: BorshSerialize,
    H: ToKey,
{
    pub(super) fn new(map: &'a TreeMap<K, V, H>) -> Self {
        Self { keys: Keys::new(&map.tree), values: &map.values }
    }
}

impl<'a, K, V, H> Iterator for Iter<'a, K, V, H>
where
    K: BorshSerialize + Ord + BorshDeserialize + Clone,
    V: BorshSerialize + BorshDeserialize,
    H: ToKey,
{
    type Item = (&'a K, &'a V);

    fn next(&mut self) -> Option<Self::Item> {
        <Self as Iterator>::nth(self, 0)
    }

    fn nth(&mut self, n: usize) -> Option<Self::Item> {
        let key = self.keys.nth(n)?;
        let entry = expect(self.values.get(key));

        Some((key, entry))
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
    H: ToKey,
{
}
impl<'a, K, V, H> FusedIterator for Iter<'a, K, V, H>
where
    K: BorshSerialize + Ord + BorshDeserialize + Clone,
    V: BorshSerialize + BorshDeserialize,
    H: ToKey,
{
}

impl<'a, K, V, H> DoubleEndedIterator for Iter<'a, K, V, H>
where
    K: BorshSerialize + Ord + BorshDeserialize + Clone,
    V: BorshSerialize + BorshDeserialize,
    H: ToKey,
{
    fn next_back(&mut self) -> Option<Self::Item> {
        <Self as DoubleEndedIterator>::nth_back(self, 0)
    }

    fn nth_back(&mut self, n: usize) -> Option<Self::Item> {
        let key = self.keys.nth_back(n)?;
        let entry = expect(self.values.get(key));

        Some((key, entry))
    }
}

fn get_entry_mut<'a, K, V, H>(map: &mut LookupMap<K, V, H>, key: &'a K) -> (&'a K, &'a mut V)
where
    K: BorshSerialize + Ord + BorshDeserialize + Clone,
    V: BorshSerialize + BorshDeserialize,
    H: ToKey,
{
    let entry = expect(map.get_mut(key));
    //* SAFETY: The lifetime can be swapped here because we can assert that the iterator
    //*         will only give out one mutable reference for every individual key in the bucket
    //*         during the iteration, and there is no overlap. This operates under the
    //*         assumption that all elements in the bucket are unique and no hash collisions.
    //*         Because we use 32 byte hashes and all keys are verified unique based on the
    //*         `TreeMap` API, this is safe.
    let value = unsafe { &mut *(entry as *mut V) };
    (key, value)
}

/// A mutable iterator over elements of a [`TreeMap`], in sorted order.
///
/// This `struct` is created by the `iter_mut` method on [`TreeMap`].
pub struct IterMut<'a, K, V, H>
where
    K: BorshSerialize + Ord + BorshDeserialize,
    V: BorshSerialize,
    H: ToKey,
{
    /// Values iterator which contains empty and filled cells.
    keys: Keys<'a, K>,
    /// Exclusive reference to underlying map to lookup values with `keys`.
    values: &'a mut LookupMap<K, V, H>,
}

impl<'a, K, V, H> IterMut<'a, K, V, H>
where
    K: BorshSerialize + Ord + BorshDeserialize,
    V: BorshSerialize,
    H: ToKey,
{
    pub(super) fn new(map: &'a mut TreeMap<K, V, H>) -> Self {
        Self { keys: Keys::new(&map.tree), values: &mut map.values }
    }
}

impl<'a, K, V, H> Iterator for IterMut<'a, K, V, H>
where
    K: BorshSerialize + Ord + BorshDeserialize + Clone,
    V: BorshSerialize + BorshDeserialize,
    H: ToKey,
{
    type Item = (&'a K, &'a mut V);

    fn next(&mut self) -> Option<Self::Item> {
        <Self as Iterator>::nth(self, 0)
    }

    fn nth(&mut self, n: usize) -> Option<Self::Item> {
        let key = self.keys.nth(n)?;
        Some(get_entry_mut(self.values, key))
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
    H: ToKey,
{
}
impl<'a, K, V, H> FusedIterator for IterMut<'a, K, V, H>
where
    K: BorshSerialize + Ord + BorshDeserialize + Clone,
    V: BorshSerialize + BorshDeserialize,
    H: ToKey,
{
}

impl<'a, K, V, H> DoubleEndedIterator for IterMut<'a, K, V, H>
where
    K: BorshSerialize + Ord + BorshDeserialize + Clone,
    V: BorshSerialize + BorshDeserialize,
    H: ToKey,
{
    fn next_back(&mut self) -> Option<Self::Item> {
        <Self as DoubleEndedIterator>::nth_back(self, 0)
    }

    fn nth_back(&mut self, n: usize) -> Option<Self::Item> {
        let key = self.keys.nth_back(n)?;
        Some(get_entry_mut(self.values, key))
    }
}

/// This function takes the query range and map them to references to nodes in the map
fn get_range_bounds<'a, Q, K>(
    tree: &'a Tree<K>,
    bounds: (Bound<&Q>, Bound<&Q>),
) -> Option<(Bound<&'a K>, Bound<&'a K>)>
where
    K: Borrow<Q> + BorshSerialize + Ord + BorshDeserialize,
    Q: ?Sized + Ord,
{
    let (min_bound, max_bound) = bounds;
    let min = match min_bound {
        Bound::Unbounded => Bound::Unbounded,
        Bound::Included(bound) => {
            if let Some(b) = tree.ceil_key(bound) {
                Bound::Included(b)
            } else {
                return None;
            }
        }
        Bound::Excluded(bound) => {
            if let Some(b) = tree.higher(bound) {
                Bound::Included(b)
            } else {
                return None;
            }
        }
    };

    let max = match max_bound {
        Bound::Unbounded => Bound::Unbounded,
        Bound::Included(bound) => {
            if let Some(b) = tree.floor_key(bound) {
                Bound::Included(b)
            } else {
                return None;
            }
        }
        Bound::Excluded(bound) => {
            if let Some(b) = tree.lower(bound) {
                Bound::Included(b)
            } else {
                return None;
            }
        }
    };

    Some((min, max))
}

//Returns true if key is out of bounds to min value
fn key_lt_bound<K>(key: &K, min: Bound<&K>) -> bool
where
    K: BorshSerialize + Ord + BorshDeserialize,
{
    match min {
        Bound::Unbounded => false,
        Bound::Excluded(a) => key <= a,
        Bound::Included(a) => key < a,
    }
}

//Returns true if key is out of bounds to max value
fn key_gt_bound<K>(key: &K, max: Bound<&K>) -> bool
where
    K: BorshSerialize + Ord + BorshDeserialize,
{
    match max {
        Bound::Unbounded => false,
        Bound::Excluded(a) => key >= a,
        Bound::Included(a) => key > a,
    }
}

fn find_min<'a, K>(
    tree: &'a Tree<K>,
    root: Option<&FreeListIndex>,
    stack_asc: &mut Vec<FreeListIndex>,
    min: Bound<&'a K>,
) -> Option<&'a K>
where
    K: BorshSerialize + Ord + BorshDeserialize,
{
    let mut curr = root;
    let mut seen: Option<&K> = None;

    while let Some(curr_idx) = curr {
        if let Some(node) = tree.node(*curr_idx) {
            if key_lt_bound(&node.key, min) {
                curr = node.rgt.as_ref();
            } else {
                seen = Some(&node.key);
                stack_asc.push(*curr_idx);
                curr = node.lft.as_ref();
            }
        } else {
            curr = None
        }
    }
    seen
}

fn find_max<'a, K>(
    tree: &'a Tree<K>,
    root: Option<&FreeListIndex>,
    stack_desc: &mut Vec<FreeListIndex>,
    max: Bound<&'a K>,
) -> Option<&'a K>
where
    K: BorshSerialize + Ord + BorshDeserialize,
{
    let mut curr = root;
    let mut seen: Option<&K> = None;

    while let Some(curr_idx) = curr {
        if let Some(node) = tree.node(*curr_idx) {
            if key_gt_bound(&node.key, max) {
                curr = node.lft.as_ref();
            } else {
                seen = Some(&node.key);
                stack_desc.push(*curr_idx);
                curr = node.rgt.as_ref();
            }
        } else {
            curr = None
        }
    }
    seen
}

//The last element in the stack is the last returned key.
//Find the next key to the last item in the stack
fn find_next_asc<'a, K>(tree: &'a Tree<K>, stack_asc: &mut Vec<FreeListIndex>) -> Option<&'a K>
where
    K: BorshSerialize + Ord + BorshDeserialize,
{
    let last_key_idx = stack_asc.pop();
    let mut seen: Option<&K> = None;
    if let Some(last_idx) = last_key_idx {
        if let Some(node) = tree.node(last_idx) {
            //If the last returned key has right node then return minimum key from the
            //tree where the right node is the root.
            seen = match node.rgt {
                Some(rgt) => find_min(tree, Some(&rgt), stack_asc, Bound::Unbounded),
                None => None,
            }
        }
    }
    //If the last returned key does not have right node then return the
    //last value in the stack.
    if seen.is_none() && !stack_asc.is_empty() {
        if let Some(result_idx) = stack_asc.last() {
            seen = tree.node(*result_idx).map(|f| &f.key);
        }
    }
    seen
}

fn find_next_desc<'a, K>(tree: &'a Tree<K>, stack_desc: &mut Vec<FreeListIndex>) -> Option<&'a K>
where
    K: BorshSerialize + Ord + BorshDeserialize,
{
    let last_key_idx = stack_desc.pop();
    let mut seen: Option<&K> = None;
    if let Some(last_idx) = last_key_idx {
        if let Some(node) = tree.node(last_idx) {
            //If the last returned key has left node then return maximum key from the
            //tree where the left node is the root.
            seen = match node.lft {
                Some(lft) => find_max(tree, Some(&lft), stack_desc, Bound::Unbounded),
                None => None,
            }
        }
    }
    //If the last returned key does not have left node then return the
    //last value in the stack.
    if seen.is_none() && !stack_desc.is_empty() {
        if let Some(result_idx) = stack_desc.last() {
            seen = tree.node(*result_idx).map(|f| &f.key);
        }
    }
    seen
}

/// An iterator over the keys of a [`TreeMap`], in sorted order.
///
/// This `struct` is created by the `keys` method on [`TreeMap`].
pub struct Keys<'a, K: 'a>
where
    K: BorshSerialize + BorshDeserialize + Ord,
{
    tree: &'a Tree<K>,
    length: u32,
    min: FindUnbounded,
    max: FindUnbounded,
    //The last element in the stack is the latest value returned by the iterator
    stack_asc: Vec<FreeListIndex>,
    stack_desc: Vec<FreeListIndex>,
}

impl<'a, K> Keys<'a, K>
where
    K: BorshSerialize + BorshDeserialize + Ord,
{
    pub(super) fn new(tree: &'a Tree<K>) -> Self {
        Self {
            tree,
            length: tree.nodes.len(),
            min: FindUnbounded::First,
            max: FindUnbounded::First,
            stack_asc: Vec::new(),
            stack_desc: Vec::new(),
        }
    }
}

impl<'a, K> Iterator for Keys<'a, K>
where
    K: BorshSerialize + BorshDeserialize + Ord,
{
    type Item = &'a K;

    fn next(&mut self) -> Option<&'a K> {
        if self.length == 0 {
            // Short circuit if all elements have been iterated.
            return None;
        }

        let next = match self.min {
            FindUnbounded::First => {
                find_min(self.tree, self.tree.root.as_ref(), &mut self.stack_asc, Bound::Unbounded)
            }
            FindUnbounded::Next => find_next_asc(self.tree, &mut self.stack_asc),
        };

        if next.is_some() {
            // Update minimum bound.
            self.min = FindUnbounded::Next;

            // Decrease count of potential elements
            self.length -= 1;
        } else {
            // No more elements to iterate, set length to 0 to avoid duplicate lookups.
            // Bounds can never be updated manually once initialized, so this can be done.
            self.length = 0;
        }

        next
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.length as usize;
        (len, Some(len))
    }

    fn count(self) -> usize {
        self.length as usize
    }
}

impl<'a, K> ExactSizeIterator for Keys<'a, K> where K: BorshSerialize + BorshDeserialize + Ord {}
impl<'a, K> FusedIterator for Keys<'a, K> where K: BorshSerialize + BorshDeserialize + Ord {}

impl<'a, K> DoubleEndedIterator for Keys<'a, K>
where
    K: BorshSerialize + Ord + BorshDeserialize,
{
    fn next_back(&mut self) -> Option<&'a K> {
        if self.length == 0 {
            // Short circuit if all elements have been iterated.
            return None;
        }

        let next = match self.max {
            FindUnbounded::First => {
                find_max(self.tree, self.tree.root.as_ref(), &mut self.stack_desc, Bound::Unbounded)
            }
            FindUnbounded::Next => find_next_desc(self.tree, &mut self.stack_desc),
        };

        if next.is_some() {
            // Update maximum bound.
            self.max = FindUnbounded::Next;

            // Decrease count of potential elements
            self.length -= 1;
        } else {
            // No more elements to iterate, set length to 0 to avoid duplicate lookups.
            // Bounds can never be updated manually once initialized, so this can be done.
            self.length = 0;
        }

        next
    }
}

/// An iterator over the keys of a [`TreeMap`], in sorted order.
///
/// This `struct` is created by the `keys` method on [`TreeMap`].
pub struct KeysRange<'a, K: 'a>
where
    K: BorshSerialize + BorshDeserialize + Ord,
{
    tree: &'a Tree<K>,
    length: u32,
    min: Find<&'a K>,
    max: Find<&'a K>,
    //The last element in the stack is the latest value returned by the iterator
    stack_asc: Vec<FreeListIndex>,
    stack_desc: Vec<FreeListIndex>,
}

impl<'a, K> KeysRange<'a, K>
where
    K: BorshSerialize + BorshDeserialize + Ord,
{
    pub(super) fn new<Q>(tree: &'a Tree<K>, bounds: (Bound<&Q>, Bound<&Q>)) -> Self
    where
        K: Borrow<Q>,
        Q: ?Sized + Ord,
    {
        if let Some((min, max)) = get_range_bounds(tree, bounds) {
            Self {
                tree,
                length: tree.nodes.len(),
                min: Find::First { bound: min },
                max: Find::First { bound: max },
                stack_asc: Vec::new(),
                stack_desc: Vec::new(),
            }
        } else {
            Self {
                tree,
                length: 0,
                min: Find::First { bound: Bound::Unbounded },
                max: Find::First { bound: Bound::Unbounded },
                stack_asc: Vec::new(),
                stack_desc: Vec::new(),
            }
        }
    }
}

impl<'a, K> Iterator for KeysRange<'a, K>
where
    K: BorshSerialize + BorshDeserialize + Ord,
{
    type Item = &'a K;

    fn next(&mut self) -> Option<&'a K> {
        if self.length == 0 {
            // Short circuit if all elements have been iterated.
            return None;
        }

        let next = match self.min {
            Find::First { bound: min } => {
                find_min(self.tree, self.tree.root.as_ref(), &mut self.stack_asc, min)
            }
            Find::Next { bound: _ } => find_next_asc(self.tree, &mut self.stack_asc),
        };

        if let Some(next) = next {
            // Check to make sure next key isn't past opposite bound.
            match self.max.into_value() {
                Bound::Included(bound) => {
                    if next.gt(bound) {
                        self.length = 0;
                        return None;
                    }
                }
                Bound::Excluded(bound) => {
                    if !next.lt(bound) {
                        self.length = 0;
                        return None;
                    }
                }
                Bound::Unbounded => (),
            }

            // Update minimum bound.
            self.min = Find::Next { bound: Bound::Excluded(next) };

            // Decrease count of potential elements
            self.length -= 1;
        } else {
            // No more elements to iterate, set length to 0 to avoid duplicate lookups.
            // Bounds can never be updated manually once initialized, so this can be done.
            self.length = 0;
        }

        next
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.length as usize;
        (0, Some(len))
    }
}

impl<'a, K> FusedIterator for KeysRange<'a, K> where K: BorshSerialize + BorshDeserialize + Ord {}

impl<'a, K> DoubleEndedIterator for KeysRange<'a, K>
where
    K: BorshSerialize + Ord + BorshDeserialize,
{
    fn next_back(&mut self) -> Option<&'a K> {
        if self.length == 0 {
            // Short circuit if all elements have been iterated.
            return None;
        }

        let next = match self.max {
            Find::First { bound: max } => {
                find_max(self.tree, self.tree.root.as_ref(), &mut self.stack_desc, max)
            }
            Find::Next { bound: _ } => find_next_desc(self.tree, &mut self.stack_desc),
        };

        if let Some(next) = next {
            // Check to make sure next key isn't past opposite bound
            match self.min.into_value() {
                Bound::Included(bound) => {
                    if next.lt(bound) {
                        self.length = 0;
                        return None;
                    }
                }
                Bound::Excluded(bound) => {
                    if !next.gt(bound) {
                        self.length = 0;
                        return None;
                    }
                }
                Bound::Unbounded => (),
            }

            // Update maximum bound.
            self.max = Find::Next { bound: Bound::Excluded(next) };

            // Decrease count of potential elements
            self.length -= 1;
        } else {
            // No more elements to iterate, set length to 0 to avoid duplicate lookups.
            // Bounds can never be updated manually once initialized, so this can be done.
            self.length = 0;
        }

        next
    }
}

/// An iterator over the values of a [`TreeMap`], in order by key.
///
/// This `struct` is created by the `values` method on [`TreeMap`].
pub struct Values<'a, K, V, H>
where
    K: BorshSerialize + Ord + BorshDeserialize,
    V: BorshSerialize,
    H: ToKey,
{
    inner: Iter<'a, K, V, H>,
}

impl<'a, K, V, H> Values<'a, K, V, H>
where
    K: BorshSerialize + Ord + BorshDeserialize,
    V: BorshSerialize,
    H: ToKey,
{
    pub(super) fn new(map: &'a TreeMap<K, V, H>) -> Self {
        Self { inner: map.iter() }
    }
}

impl<'a, K, V, H> Iterator for Values<'a, K, V, H>
where
    K: BorshSerialize + Ord + BorshDeserialize + Clone,
    V: BorshSerialize + BorshDeserialize,
    H: ToKey,
{
    type Item = &'a V;

    fn next(&mut self) -> Option<Self::Item> {
        <Self as Iterator>::nth(self, 0)
    }

    fn nth(&mut self, n: usize) -> Option<Self::Item> {
        self.inner.nth(n).map(|(_, v)| v)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }

    fn count(self) -> usize {
        self.inner.count()
    }
}

impl<'a, K, V, H> ExactSizeIterator for Values<'a, K, V, H>
where
    K: BorshSerialize + Ord + BorshDeserialize + Clone,
    V: BorshSerialize + BorshDeserialize,
    H: ToKey,
{
}
impl<'a, K, V, H> FusedIterator for Values<'a, K, V, H>
where
    K: BorshSerialize + Ord + BorshDeserialize + Clone,
    V: BorshSerialize + BorshDeserialize,
    H: ToKey,
{
}

impl<'a, K, V, H> DoubleEndedIterator for Values<'a, K, V, H>
where
    K: BorshSerialize + Ord + BorshDeserialize + Clone,
    V: BorshSerialize + BorshDeserialize,
    H: ToKey,
{
    fn next_back(&mut self) -> Option<Self::Item> {
        <Self as DoubleEndedIterator>::nth_back(self, 0)
    }

    fn nth_back(&mut self, n: usize) -> Option<Self::Item> {
        self.inner.nth_back(n).map(|(_, v)| v)
    }
}

/// A mutable iterator over values of a [`TreeMap`], in order by key.
///
/// This `struct` is created by the `values_mut` method on [`TreeMap`].
pub struct ValuesMut<'a, K, V, H>
where
    K: BorshSerialize + Ord + BorshDeserialize,
    V: BorshSerialize,
    H: ToKey,
{
    inner: IterMut<'a, K, V, H>,
}

impl<'a, K, V, H> ValuesMut<'a, K, V, H>
where
    K: BorshSerialize + Ord + BorshDeserialize,
    V: BorshSerialize,
    H: ToKey,
{
    pub(super) fn new(map: &'a mut TreeMap<K, V, H>) -> Self {
        Self { inner: map.iter_mut() }
    }
}

impl<'a, K, V, H> Iterator for ValuesMut<'a, K, V, H>
where
    K: BorshSerialize + Ord + BorshDeserialize + Clone,
    V: BorshSerialize + BorshDeserialize,
    H: ToKey,
{
    type Item = &'a mut V;

    fn next(&mut self) -> Option<Self::Item> {
        <Self as Iterator>::nth(self, 0)
    }

    fn nth(&mut self, n: usize) -> Option<Self::Item> {
        self.inner.nth(n).map(|(_, v)| v)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }

    fn count(self) -> usize {
        self.inner.count()
    }
}

impl<'a, K, V, H> ExactSizeIterator for ValuesMut<'a, K, V, H>
where
    K: BorshSerialize + Ord + BorshDeserialize + Clone,
    V: BorshSerialize + BorshDeserialize,
    H: ToKey,
{
}
impl<'a, K, V, H> FusedIterator for ValuesMut<'a, K, V, H>
where
    K: BorshSerialize + Ord + BorshDeserialize + Clone,
    V: BorshSerialize + BorshDeserialize,
    H: ToKey,
{
}

impl<'a, K, V, H> DoubleEndedIterator for ValuesMut<'a, K, V, H>
where
    K: BorshSerialize + Ord + BorshDeserialize + Clone,
    V: BorshSerialize + BorshDeserialize,
    H: ToKey,
{
    fn next_back(&mut self) -> Option<Self::Item> {
        <Self as DoubleEndedIterator>::nth_back(self, 0)
    }

    fn nth_back(&mut self, n: usize) -> Option<Self::Item> {
        self.inner.nth_back(n).map(|(_, v)| v)
    }
}

/// An iterator over a range of elements of a [`TreeMap`], in sorted order.
///
/// This `struct` is created by the `iter` method on [`TreeMap`].
pub struct Range<'a, K, V, H>
where
    K: BorshSerialize + Ord + BorshDeserialize,
    V: BorshSerialize,
    H: ToKey,
{
    keys: KeysRange<'a, K>,
    values: &'a LookupMap<K, V, H>,
}

impl<'a, K, V, H> Range<'a, K, V, H>
where
    K: BorshSerialize + Ord + BorshDeserialize,
    V: BorshSerialize,
    H: ToKey,
{
    pub(super) fn new<Q>(map: &'a TreeMap<K, V, H>, bounds: (Bound<&Q>, Bound<&Q>)) -> Self
    where
        K: Borrow<Q>,
        Q: ?Sized + Ord,
    {
        Self { keys: KeysRange::new(&map.tree, bounds), values: &map.values }
    }
}

impl<'a, K, V, H> Iterator for Range<'a, K, V, H>
where
    K: BorshSerialize + Ord + BorshDeserialize + Clone,
    V: BorshSerialize + BorshDeserialize,
    H: ToKey,
{
    type Item = (&'a K, &'a V);

    fn next(&mut self) -> Option<Self::Item> {
        <Self as Iterator>::nth(self, 0)
    }

    fn nth(&mut self, n: usize) -> Option<Self::Item> {
        let key = self.keys.nth(n)?;
        let entry = expect(self.values.get(key));

        Some((key, entry))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.keys.size_hint()
    }
}

impl<'a, K, V, H> FusedIterator for Range<'a, K, V, H>
where
    K: BorshSerialize + Ord + BorshDeserialize + Clone,
    V: BorshSerialize + BorshDeserialize,
    H: ToKey,
{
}

impl<'a, K, V, H> DoubleEndedIterator for Range<'a, K, V, H>
where
    K: BorshSerialize + Ord + BorshDeserialize + Clone,
    V: BorshSerialize + BorshDeserialize,
    H: ToKey,
{
    fn next_back(&mut self) -> Option<Self::Item> {
        <Self as DoubleEndedIterator>::nth_back(self, 0)
    }

    fn nth_back(&mut self, n: usize) -> Option<Self::Item> {
        let key = self.keys.nth_back(n)?;
        let entry = expect(self.values.get(key));

        Some((key, entry))
    }
}

/// A mutable iterator over a range of elements of a [`TreeMap`], in sorted order.
///
/// This `struct` is created by the `iter_mut` method on [`TreeMap`].
pub struct RangeMut<'a, K, V, H>
where
    K: BorshSerialize + Ord + BorshDeserialize,
    V: BorshSerialize,
    H: ToKey,
{
    keys: KeysRange<'a, K>,
    /// Exclusive reference to underlying map to lookup values with `keys`.
    values: &'a mut LookupMap<K, V, H>,
}

impl<'a, K, V, H> RangeMut<'a, K, V, H>
where
    K: BorshSerialize + Ord + BorshDeserialize,
    V: BorshSerialize,
    H: ToKey,
{
    pub(super) fn new<Q>(map: &'a mut TreeMap<K, V, H>, bounds: (Bound<&Q>, Bound<&Q>)) -> Self
    where
        K: Borrow<Q>,
        Q: ?Sized + Ord,
    {
        Self { keys: KeysRange::new(&map.tree, bounds), values: &mut map.values }
    }
}

impl<'a, K, V, H> Iterator for RangeMut<'a, K, V, H>
where
    K: BorshSerialize + Ord + BorshDeserialize + Clone,
    V: BorshSerialize + BorshDeserialize,
    H: ToKey,
{
    type Item = (&'a K, &'a mut V);

    fn next(&mut self) -> Option<Self::Item> {
        <Self as Iterator>::nth(self, 0)
    }

    fn nth(&mut self, n: usize) -> Option<Self::Item> {
        let key = self.keys.nth(n)?;
        Some(get_entry_mut(self.values, key))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.keys.size_hint()
    }
}

impl<'a, K, V, H> FusedIterator for RangeMut<'a, K, V, H>
where
    K: BorshSerialize + Ord + BorshDeserialize + Clone,
    V: BorshSerialize + BorshDeserialize,
    H: ToKey,
{
}

impl<'a, K, V, H> DoubleEndedIterator for RangeMut<'a, K, V, H>
where
    K: BorshSerialize + Ord + BorshDeserialize + Clone,
    V: BorshSerialize + BorshDeserialize,
    H: ToKey,
{
    fn next_back(&mut self) -> Option<Self::Item> {
        <Self as DoubleEndedIterator>::nth_back(self, 0)
    }

    fn nth_back(&mut self, n: usize) -> Option<Self::Item> {
        let key = self.keys.nth_back(n)?;
        Some(get_entry_mut(self.values, key))
    }
}

#[derive(Debug, Copy, Clone)]
enum Find<K> {
    /// Find the first element based on bound.
    First { bound: Bound<K> },
    /// Find the next element from current pos
    Next { bound: Bound<K> },
}

impl<K> Find<K> {
    fn into_value(self) -> Bound<K> {
        match self {
            Find::First { bound } => bound,
            Find::Next { bound } => bound,
        }
    }
}

#[derive(BorshDeserialize, BorshSerialize, Debug, Copy, Clone)]
enum FindUnbounded {
    /// Find the first element in the given root
    First,
    /// Find the next element from current pos
    Next,
}
