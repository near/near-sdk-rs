use super::{RedBlackTree, Direction, RedBlackNode, RedBlackNodeValue};

use borsh::{BorshDeserialize, BorshSerialize};
use std::ops::{RangeBounds, Bound};

pub struct RedBlackTreeIter<'a, T> {
    tree: &'a RedBlackTree<T>,
    visited: Vec<RedBlackNode<T>>,
    root: Option<RedBlackNode<T>>,
}

impl<'a, T> RedBlackTreeIter<'a, T> 
where
    T: BorshSerialize + BorshDeserialize + RedBlackNodeValue + std::fmt::Debug,
    <T as RedBlackNodeValue>::OrdValue: std::fmt::Debug
{
    pub fn new(tree: &'a RedBlackTree<T>) -> Self {
        Self {
            tree,
            visited: vec!(),
            root: tree.get_root(),
        }
    }

    fn direction(asc: bool) -> Direction {
        if asc {
            Direction::Left
        } else {
            Direction::Right
        }
    }

    fn get_leaf_child(&mut self, node: RedBlackNode<T>, direction: &Direction) -> RedBlackNode<T> {
        let mut last_node = node;

        while let Some(leaf_child_node) = self.tree.get_child(&last_node, direction) {
            self.visited.push(last_node); 
            last_node = leaf_child_node;
        }

        last_node
    }

    fn get_next_node(&mut self, node: RedBlackNode<T>, direction: &Direction) -> RedBlackNode<T> {
        if node.has_child(direction) {
            self.get_leaf_child(node, direction)
        } else {
            node
        }
    }

    // comments correspond to ascending traversal of the tree. implementation is direction generic,
    // so effectively comments describe iteration where asc=true, direction=Left
    pub fn next(&mut self, asc: bool) -> Option<RedBlackNode<T>> {
        let direction = Self::direction(asc);
        let next_node = self.visited.pop();
        
        // if this is the first call to next(), visit the left subtree of the root of the tree.
        // then return the leftmost child (leaf node) of the tree 
        // NOTE: this iterator cannot be re-used. it will forever return None after finishing iteration
        if let Some(root_node) = self.root.take() {
            let node = self.get_next_node(root_node, &direction);
            Some(node)
        } 
        // if there is some node that has been visited, visit its right child's left subtree. 
        // then return the visited node
        else if let Some(node) = next_node {
            if let Some(right_child_node) = self.tree.get_child(&node, &direction.opposite()) {
                let n = self.get_next_node(right_child_node, &direction);
                self.visited.push(n);
            }
            Some(node)
        }
        // if tree is no root, or there are no nodes left return None
        else {
            None
        }
    }
}

impl<'a, T> Iterator for RedBlackTreeIter<'a, T> 
where
    T: BorshSerialize + BorshDeserialize + RedBlackNodeValue + std::fmt::Debug,
    <T as RedBlackNodeValue>::OrdValue: std::fmt::Debug
{
    type Item = RedBlackNode<T>;

    /// in order traversal in ascending order
    fn next(&mut self) -> Option<Self::Item> {
        Self::next(self, true)
    }
}

// FIXME if next() and next_back() are called interchangeably, the visited vec of RedBlackTreeIter will become corrupted! 
// can think about how to address this to get intended behavior...
impl<'a, T> DoubleEndedIterator for RedBlackTreeIter<'a, T> 
where
    T: BorshSerialize + BorshDeserialize + RedBlackNodeValue + std::fmt::Debug,
    <T as RedBlackNodeValue>::OrdValue: std::fmt::Debug
{
    /// in order traversal in descending order
    fn next_back(&mut self) -> Option<Self::Item> {
        Self::next(self, false)
    }
}

impl<'a, T> std::iter::FusedIterator for RedBlackTreeIter<'a, T> 
where
    T: BorshSerialize + BorshDeserialize + RedBlackNodeValue + std::fmt::Debug,
    <T as RedBlackNodeValue>::OrdValue: std::fmt::Debug
{}

pub struct RedBlackTreeRange<'a, T, R> {
    iter: RedBlackTreeIter<'a, T>,
    init: bool,
    visited: Vec<RedBlackNode<T>>,
    range: R
}

impl<'a, T, R> RedBlackTreeRange<'a, T, R> 
where
    T: BorshSerialize + BorshDeserialize + RedBlackNodeValue + std::fmt::Debug,
    <T as RedBlackNodeValue>::OrdValue: std::fmt::Debug,
    R: RangeBounds<T>
{
    pub fn new(tree: &'a RedBlackTree<T>, range: R) -> Self {
        Self {
            iter: RedBlackTreeIter {
                root: None,
                tree,
                visited: vec!()
            },
            init: false,
            visited: vec!(),
            range
        }
    }

    fn visit_first_node(&mut self, asc: bool) {
        let initial_bound = if asc { self.range.start_bound() } else { self.range.end_bound() };
        let first_node = match initial_bound {
            Bound::Included(value) => if asc { self.iter.tree.floor(value) } else { self.iter.tree.ceil(value) },
            Bound::Excluded(value) => if asc { self.iter.tree.above(value) } else { self.iter.tree.below(value) },
            Bound::Unbounded => self.iter.tree.get_root()
        };

        if let Some(parent) = first_node {
            self.visited.push(parent);
        }
    }

    fn next(&mut self, asc: bool) -> Option<RedBlackNode<T>> {
        use Direction::*;
        let direction = if asc { Right } else { Left };
        if let Some(node) = self.visited.pop() {
            self.iter.root = self.iter.tree.get_child(&node, &direction);
            if let Some(parent) = self.iter.tree.get_parent(&node) {
                self.visited.push(parent);
            }
            Some(node)
        } else {
            None
        }
    }

    fn next_node(&mut self, asc: bool) -> Option<RedBlackNode<T>> {
        let next_node = if !self.init {
            self.visit_first_node(asc);
            self.init = true;
            self.next(asc)
        } else {
            let iter_next_node = if asc {
                <RedBlackTreeIter<T> as Iterator>::next(&mut self.iter)
            } else {
                <RedBlackTreeIter<T> as DoubleEndedIterator>::next_back(&mut self.iter)
            };
            
            iter_next_node.or_else(|| {
                while let Some(node) = self.next(asc) {
                    if self.range.contains(&node.value) {
                        return Some(node)
                    }
                }
                None
            })
        };
        
        next_node.filter(|node| self.range.contains(&node.value))
    }
}

impl<'a, T, R> Iterator for RedBlackTreeRange<'a, T, R> 
where
    T: BorshSerialize + BorshDeserialize + RedBlackNodeValue + std::fmt::Debug,
    <T as RedBlackNodeValue>::OrdValue: std::fmt::Debug,
    R: RangeBounds<T>
{
    type Item = RedBlackNode<T>;

    /// in order traversal in ascending order with bounds
    fn next(&mut self) -> Option<Self::Item> {
        self.next_node(true)
    }
}

impl<'a, T, R> DoubleEndedIterator for RedBlackTreeRange<'a, T, R> 
where
    T: BorshSerialize + BorshDeserialize + RedBlackNodeValue + std::fmt::Debug,
    <T as RedBlackNodeValue>::OrdValue: std::fmt::Debug,
    R: RangeBounds<T>
{
    /// in order traversal in descending order with bounds
    fn next_back(&mut self) -> Option<Self::Item> {
        self.next_node(false)
    }
}

impl<'a, T, R> std::iter::FusedIterator for RedBlackTreeRange<'a, T, R> 
where
    T: BorshSerialize + BorshDeserialize + RedBlackNodeValue + std::fmt::Debug,
    <T as RedBlackNodeValue>::OrdValue: std::fmt::Debug,
    R: RangeBounds<T>
{}