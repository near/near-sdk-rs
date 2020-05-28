use super::{Set, TreeSet};
use crate::collections::next_trie_id;
use crate::env;

use borsh::{BorshDeserialize, BorshSerialize};
use std::{
    marker::PhantomData,
    sync::Mutex,
    collections::{HashMap, HashSet},
    ops::{RangeBounds, Bound},
};

// const ERR_INCONSISTENT_STATE: &[u8] = b"The collection is an inconsistent state. Did previous smart contract execution terminate unexpectedly?";
const ERR_ELEMENT_DESERIALIZATION: &[u8] = b"Cannot deserialize element";
const ERR_ELEMENT_SERIALIZATION: &[u8] = b"Cannot serialize element";

type EnvStorageKey = Vec<u8>;

#[derive(Debug)]
pub enum DoubleBlackNodeCase {
    LeftSiblingIsRed,
    NodeIsLeftChild,
    NodeIsRightChild
}

#[derive(Debug)]
pub enum Direction {
    Left,
    Right
}

impl Direction {
    fn opposite(&self) -> Self {
        use Direction::*;
        match self {
            Left => Right,
            Right => Left
        }
    }
}

// #[derive(BorshSerialize, BorshDeserialize, Clone)]
// pub struct RawRedBlackNode<T> {
//     color: bool, // 0 Red, 1 Black
//     is_right_child: bool,
//     // key: EnvStorageKey,
//     parent_key: Option<EnvStorageKey>,
//     left_key: Option<EnvStorageKey>,
//     right_key: Option<EnvStorageKey>,
//     value: T
// }

#[derive(BorshSerialize, BorshDeserialize, Clone)]
pub struct RedBlackNode<T> {
    color: u8,
    is_right_child: bool,
    key: EnvStorageKey,
    parent_key: Option<EnvStorageKey>,
    left_key: Option<EnvStorageKey>,
    right_key: Option<EnvStorageKey>,
    value: T
}

pub trait RedBlackNodeValue: Ord {
    type OrdValue: Ord;

    fn ord_value(&self) -> &Self::OrdValue;
}

// Need specialization in stable rust for this to work as intended
// impl<T: Ord> RedBlackNodeValue for T {
//     type OrdValue = Self;

//     fn ord_value(&self) -> &Self::OrdValue {
//         self
//     }
// }

// impl<T> RedBlackNodeValue for T {
//     type OrdValue = Self;

//     fn ord_value(&self) -> &Self::OrdValue {
//         self
//     }
// }

impl RedBlackNodeValue for u64 {
    type OrdValue = Self;

    fn ord_value(&self) -> &Self::OrdValue {
        self
    }
}


impl<T> Ord for RedBlackNode<T> 
where
    T: RedBlackNodeValue
{
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.value.ord_value().cmp(&other.value.ord_value())
    }
}

impl<T> PartialOrd for RedBlackNode<T> 
where
    T: RedBlackNodeValue
{
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.value.ord_value().cmp(&other.value.ord_value()))
    }
}

impl<T> PartialEq for RedBlackNode<T> 
where
    T: RedBlackNodeValue
{
    fn eq(&self, other: &Self) -> bool {
        self.value.ord_value() == other.value.ord_value()
    }
}

impl<T> Eq for RedBlackNode<T> 
where
    T: RedBlackNodeValue
{}

impl<T> RedBlackNode<T> {
    pub fn key(&self) -> &EnvStorageKey {
        &self.key
    }

    pub fn left_node_key(&self) -> Option<&EnvStorageKey> {
        self.left_key.as_ref()
    }

    pub fn right_node_key(&self) -> Option<&EnvStorageKey> {
        self.right_key.as_ref()
    }

    pub fn is_root(&self) -> bool {
        self.parent_key.is_none()
    }

    pub fn is_black(&self) -> bool {
        self.color > 0
    }

    pub fn is_double_black(&self) -> bool {
        self.color == 2
    }

    pub fn is_red(&self) -> bool {
        self.color == 0
    }

    pub fn is_left_child(&self) -> bool {
        !self.is_right_child
    }

    pub fn is_right_child(&self) -> bool {
        self.is_right_child
    }

    pub fn child_direction(&self) -> Direction {
        use Direction::*;
        if self.is_right_child { Right } else { Left }
    }

    pub fn has_right_child(&self) -> bool {
        self.right_key.is_some()
    }

    pub fn has_left_child(&self) -> bool {
        self.left_key.is_some()
    }

    pub fn has_child(&self, direction: &Direction) -> bool {
        use Direction::*;
        match direction {
            Left => self.has_left_child(),
            Right => self.has_right_child()
        }
    }

    pub fn set_right_child(&mut self, node: Option<&mut RedBlackNode<T>>) {
        self.right_key = node.as_ref().map(|child_node| child_node.key().clone());
        if let Some(child_node) = node {
            child_node.parent_key = Some(self.key().clone());
            child_node.is_right_child = true;
        }
    }

    pub fn set_left_child(&mut self, node: Option<&mut RedBlackNode<T>>) {
        self.left_key = node.as_ref().map(|child_node| child_node.key().clone());
        if let Some(child_node) = node {
            child_node.parent_key = Some(self.key().clone());
            child_node.is_right_child = false;
        }
    }

    pub fn set_child(&mut self, node: Option<&mut RedBlackNode<T>>, direction: &Direction) {
        use Direction::*;
        match direction {
            Left => self.set_left_child(node),
            Right => self.set_right_child(node)
        }
    }
}


impl<T> RedBlackNode<T> 
where 
    T: RedBlackNodeValue
{

}

#[derive(Default)]
struct EnvStorageCache {
    prefix: EnvStorageKey,
    cache: Mutex<HashMap<EnvStorageKey, Vec<u8>>>,
    dirty_keys: Mutex<HashSet<EnvStorageKey>>
}

impl EnvStorageCache {
    pub fn new(prefix: EnvStorageKey) -> Self {
        Self {
            prefix,
            cache: Mutex::new(HashMap::new()),
            dirty_keys: Mutex::new(HashSet::new())
        }
    }

    pub fn read(&self, key: &EnvStorageKey) -> Option<Vec<u8>> {
        let mut cache = self.cache.lock().expect("lock is not poisoned");
        if !cache.contains_key(key) {
            if let Some(data) = self.get_node_raw(key) {
                cache.insert(key.clone(), data);
            }
        }
        cache.get(key).map(|v| v.clone())
    }

    pub fn update(&self, key: &EnvStorageKey, value: &Vec<u8>) {
        let mut cache = self.cache.lock().expect("lock is not poisoned");
        cache.insert(key.clone(), value.clone());
        let mut dirty_keys = self.dirty_keys.lock().expect("lock is not poisoned");
        dirty_keys.insert(key.clone());
    }

    pub fn insert(&self, key: &EnvStorageKey, value: &Vec<u8>) {
        let mut cache = self.cache.lock().expect("lock is not poisoned");
        cache.insert(key.clone(), value.clone());
        self.insert_node_raw(key, value);
    }

    pub fn delete(&self, key: &EnvStorageKey) {
        self.delete_node_raw(key)
    }

    pub fn clear(&self) {
        let mut cache = self.cache.lock().expect("lock is not poisoned");
        for key in self.dirty_keys.lock().expect("lock is not poisoned").drain() {
            let node = cache.get(&key).expect("value must exist");
            self.update_node_raw(&key, node);
        }
        cache.drain();
    }

    fn get_node_raw(&self, key: &EnvStorageKey) -> Option<Vec<u8>> {
        let lookup_key = [&self.prefix, key.as_slice()].concat();
        // println!("RAW: looking up node for key {:?}", key);
        env::storage_read(&lookup_key)
    }

    fn insert_node_raw(&self, key: &EnvStorageKey, node: &Vec<u8>) {
        let lookup_key = [&self.prefix, key.as_slice()].concat();
        // println!("RAW: inserting node for key {:?}", key);
        if env::storage_write(&lookup_key, node) {
            panic!("insert node raw panic");
            // env::panic(ERR_INCONSISTENT_STATE) // Node should not exist already
        }
    }

    fn update_node_raw(&self, key: &EnvStorageKey, node: &Vec<u8>) {
        let lookup_key = [&self.prefix, key.as_slice()].concat();
        // println!("RAW: updating node for key {:?}", key);
        if !env::storage_write(&lookup_key, node) { 
            panic!("update node raw panic");
            // env::panic(ERR_INCONSISTENT_STATE) // Node should already exist
        }
    }

    fn delete_node_raw(&self, key: &EnvStorageKey) {
        let lookup_key = [&self.prefix, key.as_slice()].concat();
        if !env::storage_remove(&lookup_key) { 
            panic!("delete node raw panic. node key={:?}", key);
            // env::panic(ERR_INCONSISTENT_STATE) // Node should already exist
        }
    }
}

#[derive(BorshSerialize, BorshDeserialize)]
pub struct RedBlackTree<T> {
    prefix: EnvStorageKey,
    len: u64,
    root_key: Option<EnvStorageKey>,
    // TODO store indices that have been removed. 
    #[borsh_skip]
    node_value: PhantomData<T>,
    #[borsh_skip]
    cache: EnvStorageCache
}

impl<T> RedBlackTree<T> {
    pub fn new(prefix: EnvStorageKey) -> Self {
        Self {
            len: 0,
            root_key: None,
            node_value: PhantomData,
            cache: EnvStorageCache::new(prefix.clone()),
            prefix
        }
    }

    /// Init function used by borsh immediately after deserialization. Sets prefix on EnvStorageCache
    pub fn init(&mut self) {
        self.cache.prefix = self.prefix.clone();
    }
}

impl<T> Default for RedBlackTree<T> {
    fn default() -> Self {
        Self::new(next_trie_id())
    }
}

impl<T> RedBlackTree<T> 
where 
    T: BorshSerialize + BorshDeserialize + RedBlackNodeValue + std::fmt::Debug,
    <T as RedBlackNodeValue>::OrdValue: std::fmt::Debug
{   
    /// Returns the number of nodes in the tree
    pub fn len(&self) -> u64 {
        self.len
    }

    /// Returns true if the tree contains the passed in value
    pub fn contains(&self, value: &T) -> bool {
        self.find_parent(value.ord_value()).map_or(false, |node| node.value.ord_value() == value.ord_value())
    }

    /// Returns some value if it exists using the value's OrdValue to test equality,
    /// otherwise returns none if the value is not found in the tree
    pub fn get(&self, value: &<T as RedBlackNodeValue>::OrdValue) -> Option<T> {
        self.find_parent(value).map(|node| node.value)
    }

    /// Checks that every node is the tree satifies the following property:
    /// if node's left child's color is black, then the node's right child's color is also black
    pub fn validate_left_leaning_invariant(&self) -> bool {
        self.iter_nodes().all(|node| self.assert_left_leaning_invariant(&node))
    }

    /// Creates an iterator that visits values in the tree
    pub fn iter<'a>(&'a self) -> impl DoubleEndedIterator<Item=T> + 'a {
        self.iter_nodes().map(|node| node.value)
    }

    pub fn range<'a, R: RangeBounds<T> + 'a>(&'a self, range: R) -> impl DoubleEndedIterator<Item=T> + 'a {
        self.range_nodes(range).map(|node| node.value)
    }

    /// Removes all values from the tree
    pub fn clear(&mut self) {
        for node in self.iter_nodes() {
            self.cache.delete(node.key());
        }
        self.len = 0;
        self.root_key = None;
        self.update();
    }

    fn iter_nodes<'a>(&'a self) -> impl DoubleEndedIterator<Item=RedBlackNode<T>> + 'a {
        RedBlackTreeIter::new(self)
    }

    fn range_nodes<'a, R: RangeBounds<T> + 'a>(&'a self, range: R) -> impl DoubleEndedIterator<Item=RedBlackNode<T>> + 'a {
        RedBlackTreeRange::new(self, range)
    }

    fn update(&self) {
        env::storage_write(&self.prefix, &self.try_to_vec().expect("serialization works"));
    }

    fn deserialize_node(raw_node: &[u8]) -> RedBlackNode<T> {
        match RedBlackNode::try_from_slice(&raw_node) {
            Ok(x) => x,
            Err(_) => env::panic(ERR_ELEMENT_DESERIALIZATION),
        }
    }

    fn serialize_node(node: &RedBlackNode<T>) -> Vec<u8> {
        match node.try_to_vec() {
            Ok(x) => x,
            Err(_) => env::panic(ERR_ELEMENT_SERIALIZATION),
        }
    }

    fn get_node(&self, key: &EnvStorageKey) -> Option<RedBlackNode<T>> {
        self.cache.read(key).map(|raw_node| Self::deserialize_node(&raw_node))
    }

    fn insert_node(&self, node: &RedBlackNode<Option<T>>, value: &T) {
        // Taking advantage of Borsh implementation details... FIXME
        let mut serialized_value = node.try_to_vec().expect("FIXME handle serialization error");
        serialized_value.pop(); // removes last byte of the array, corresponding to the None value
        value.serialize(&mut serialized_value).expect("FIXME handle serialization error");
        self.cache.insert(node.key(), &serialized_value)
    }

    fn update_node(&self, node: &RedBlackNode<T>) {
        self.cache.update(node.key(), &Self::serialize_node(node))
    }

    fn delete_node(&mut self, node: &RedBlackNode<T>) {
        self.cache.delete(node.key());
        self.len -= 1;
    }

    fn assert_left_leaning_invariant(&self, node: &RedBlackNode<T>) -> bool {
        // check left-leaning invariant
        let left_child_is_black = self.get_left_child(&node).map_or(true, |n| n.is_black());
        if left_child_is_black {
            let right_child_is_black = self.get_right_child(&node).map_or(true, |n| n.is_black());
            right_child_is_black
        } else {
            true
        }
    }

    fn new_node_key(&mut self) -> EnvStorageKey {
        self.len += 1;
        self.len.clone().to_le_bytes().to_vec()
    }

    //
    // convenience methods for reading nodes
    //

    fn get_root(&self) -> Option<RedBlackNode<T>> {
        self.root_key.as_ref().map(|key| self.get_node(key).expect("root node must exist"))
    }

    fn get_parent(&self, node: &RedBlackNode<T>) -> Option<RedBlackNode<T>> {
        node.parent_key.as_ref().map(|key| self.get_node(key)).flatten()
    }

    fn get_left_child(&self, node: &RedBlackNode<T>) -> Option<RedBlackNode<T>> {
        node.left_node_key().map(|key| self.get_node(key)).flatten()
    }

    fn get_right_child(&self, node: &RedBlackNode<T>) -> Option<RedBlackNode<T>> {
        node.right_node_key().map(|key| self.get_node(key)).flatten()
    }

    pub fn get_child(&self, node: &RedBlackNode<T>, direction: &Direction) -> Option<RedBlackNode<T>> {
        use Direction::*;
        match direction {
            Left => self.get_left_child(node),
            Right => self.get_right_child(node)
        }
    }

    fn get_leaf_child(&self, node: &RedBlackNode<T>, direction: &Direction) -> Option<RedBlackNode<T>> {
        let mut leaf_child = None;
        while let Some(child) = self.get_child(leaf_child.as_ref().unwrap_or(node), direction) {
            leaf_child = Some(child);
        }
        leaf_child
    }

    /// Returns the smallest value that is strictly greater than value given as the parameter
    fn above(&self, value: &T) -> Option<RedBlackNode<T>> {
        use std::cmp::Ordering::*;
        let mut root = self.get_root();
        let mut parent = None; 
        
        while let Some(parent_node) = root {
            root = match parent_node.value.ord_value().cmp(value.ord_value()) {
                Greater => self.get_left_child(&parent_node),
                Less | Equal => self.get_right_child(&parent_node),
            };
            parent = Some(parent_node);
        }
        
        if let Some(node) = &parent {
            if &node.value <= value {
                return None
            }
        }

        parent
    }

    /// Returns the largest value that is strictly less than value given as the parameter
    fn below(&self, value: &T) -> Option<RedBlackNode<T>> {
        use std::cmp::Ordering::*;
        let mut root = self.get_root();
        let mut parent = None; 
            
        while let Some(parent_node) = root {
            root = match parent_node.value.ord_value().cmp(value.ord_value()) {
                Greater | Equal => self.get_left_child(&parent_node),
                Less => self.get_right_child(&parent_node),
            };
            parent = Some(parent_node);
        }
        
        if let Some(node) = &parent {
            if &node.value >= value {
                return None
            }
        }

        parent
    }

    /// Returns the largest value that is greater or equal to value given as the parameter
    fn ceil(&self, value: &T) -> Option<RedBlackNode<T>> {
        use std::cmp::Ordering::*;
        let mut root = self.get_root();
        let mut parent = None; 
        
        while let Some(parent_node) = root {
            root = match parent_node.value.ord_value().cmp(value.ord_value()) {
                Greater => self.get_left_child(&parent_node),
                Less | Equal => self.get_right_child(&parent_node),
            };
            parent = Some(parent_node);
        }
        
        if let Some(node) = &parent {
            if &node.value < value {
                return None
            }
        }

        parent
    }
    
    /// Returns the smallest value that is greater or equal to value given as the parameter
    fn floor(&self, value: &T) -> Option<RedBlackNode<T>> {
        use std::cmp::Ordering::*;
        let mut root = self.get_root();
        let mut parent = None; 
            
        while let Some(parent_node) = root {
            root = match parent_node.value.ord_value().cmp(value.ord_value()) {
                Greater | Equal => self.get_left_child(&parent_node),
                Less => self.get_right_child(&parent_node),
            };
            parent = Some(parent_node);
        }
        
        if let Some(node) = &parent {
            if &node.value >= value {
                return None
            }
        }

        parent
    }

    //
    // core logic of red black tree
    //

    /// Returns false if the value is already present in the tree,
    /// otherwise insert that value into the tree and returns true
    pub fn insert(&mut self, value: &T) -> bool { //Option<T> {
        let child = match self.find_parent(value.ord_value()) { // O(logN)
            Some(parent) => self.add_child(parent, value), // O(1)
            None => { // O(1)
                // println!("inserting new root");
                // new root
                let key = self.new_node_key();
                let root_node = RedBlackNode {
                    value: None,
                    key,
                    is_right_child: false, // arbitrary default
                    color: 1, // Black
                    parent_key: None,
                    left_key: None,
                    right_key: None
                };
                self.insert_node(&root_node, value); 
                self.root_key = Some(root_node.key().clone());
                // Ok(root_node)
                Some(self.get_node(root_node.key()).expect("root node exists"))
            }
        };

        let existing_value = match child {
            Some(child_node) => {
                // O(logN)
                self.add_red_node(child_node); // does nothing if child is already black
                true
            },
            None => false
            // Err(old_value) => Some(old_value)
        };

        // write updates to env storage
        self.cache.clear();
        self.update();
        existing_value
    }

    fn find_parent(&self, child_value: &<T as RedBlackNodeValue>::OrdValue) -> Option<RedBlackNode<T>> {
        // println!("TREE: find_parent()");
        use std::cmp::Ordering::*;
        let mut root = self.get_root();
        let mut parent = None; 
        
        while let Some(parent_node) = root {
            // println!("TREE: find_parent() : {:?} gtlte {:?} = {:?}", child_value, parent_node.value, child_value.cmp(&parent_node.value));
            root = match child_value.cmp(&parent_node.value.ord_value()) {
                Greater => self.get_right_child(&parent_node),
                Less => self.get_left_child(&parent_node),
                Equal => None
            };
            // println!("TREE: find_parent() : new root = {:?}", root.as_ref().map(|n| n.key()));
            parent = Some(parent_node);
        }
        // println!("TREE: find_parent() : parent of {:?} is {:?}, parent key {:?}", child_value, parent.as_ref().map(|p| &p.value), parent.as_ref().map(|p| p.key()));
        parent
    }

    fn add_child(&mut self, mut parent: RedBlackNode<T>, child_value: &T) -> Option<RedBlackNode<T>> { //Result<RedBlackNode<T>, T> {
        // println!("TREE: add_child() parent key {:?}", parent.key());
        use std::cmp::Ordering::*;
        match child_value.ord_value().cmp(&parent.value.ord_value()) {
            Greater => {
                // println!("TREE: add_child() adding right child -->");
                let key = self.new_node_key();
                parent.right_key = Some(key.clone());
                let child_node = RedBlackNode {
                    value: None,
                    key,
                    parent_key: Some(parent.key().clone()),
                    is_right_child: true,
                    color: 0, // Red
                    left_key: None,
                    right_key: None
                };
                self.insert_node(&child_node, child_value);
                self.update_node(&parent);
                // Ok(self.get_node(child_node.key()).expect("child exists"))
                Some(self.get_node(child_node.key()).expect("child exists"))

            },
            Less => {
                // println!("TREE: add_child() adding left child <--");
                let key = self.new_node_key();
                parent.left_key = Some(key.clone());
                let child_node = RedBlackNode {
                    value: None,
                    key,
                    parent_key: Some(parent.key().clone()),
                    is_right_child: false,
                    color: 0, // Red
                    left_key: None,
                    right_key: None
                };
                self.insert_node(&child_node, child_value);
                self.update_node(&parent);
                // Ok(self.get_node(child_node.key()).expect("child exists"))
                Some(self.get_node(child_node.key()).expect("child exists"))
            },
            Equal => {
                // FIXME this is tough to do with &T as input... 
                // the value already exists in the tree
                // return the old value
                // std::mem::swap(&mut child_value, &mut parent.value);
                // self.update_node(&parent);
                // Err(child_value)
                None
            }
        }
    }

    fn add_red_node(&mut self, child_node: RedBlackNode<T>) { // O(logN)
        use Direction::*;
        // println!("TREE: add_red_node() key {:?}", child_node.key());
        let mut node = child_node;
        while node.is_red() {
            if let Some(mut parent_node) = self.get_parent(&node) {
                let left_child = if node.is_right_child {
                    // only read from env storage if necessary
                    self.get_left_child(&parent_node)
                } else {
                    Some(node)
                };
    
                let left_child_is_black = left_child.map_or(true, |left_child_node| left_child_node.is_black());
                if left_child_is_black {
                    self.rotate(&Left, &mut parent_node, true);
    
                    node = parent_node;
                    parent_node = self.get_parent(&node).expect("parent must exist");
                }
    
                if parent_node.is_black() {
                    break
                }
                
                let mut grand_parent_node = self.get_parent(&parent_node).expect("parent must exist");
                let right_grand_child = self.get_right_child(&grand_parent_node);
                
                let right_grand_child_is_black = right_grand_child.map_or(true, |right_grand_child_node| right_grand_child_node.is_black());
                if right_grand_child_is_black {
                    self.rotate(&Right, &mut grand_parent_node, true);
                    break
                }
    
                self.push_black(&mut grand_parent_node);
    
                node = grand_parent_node;
            } else {
                // This node is the root node, color it black
                node.color = 1;
                self.update_node(&node);
            }
        }
    }

    fn rotate(&mut self, direction: &Direction, pivot_node: &mut RedBlackNode<T>, swap_colors: bool) { // O(1)
        // println!("TREE: rotate() {:?} pivot node value={:?}", direction, pivot_node.value);
        use Direction::*;
        match self.get_child(pivot_node, &direction.opposite()) {
            Some(mut child_node) => {
                if swap_colors {
                    let parent_color = pivot_node.color;
                    let child_color = child_node.color;
                
                    pivot_node.color = child_color;
                    child_node.color = parent_color;
                }

                // Replace pivot node with its right child
                if let Some(mut pivot_parent_node) = self.get_parent(&pivot_node) {
                    pivot_parent_node.set_child(Some(&mut child_node), &pivot_node.child_direction());
                    self.update_node(&pivot_parent_node);
                } else {
                    // pivot node parent_key is none -- assert this! FIXME
                    child_node.parent_key = pivot_node.parent_key.clone();
                    child_node.is_right_child = pivot_node.is_right_child;
                }
                
                // Replace pivot node's right child with former right child's left child
                if let Some(mut grand_child_node) = self.get_child(&child_node, direction) {
                    pivot_node.set_child(Some(&mut grand_child_node), &direction.opposite());
                    // println!("TREE: rotate() {:?} grand child updating", direction);
                    self.update_node(&grand_child_node);
                } else {
                    match direction {
                        Left => pivot_node.right_key = None,
                        Right => pivot_node.left_key = None
                    }
                }

                // Replace pivot node's former right child's left child with pivot node
                child_node.set_child(Some(pivot_node), direction);

                // println!("TREE: rotate() {:?} pivot node updating", direction);
                self.update_node(pivot_node);
                // println!("TREE: rotate() {:?} child node updating", direction);
                self.update_node(&child_node);

                // check if child_node is new root
                if child_node.is_root() {
                    self.root_key = Some(child_node.key().clone())
                }
            },
            None => {
                panic!("rotate {:?} panic. {:?} has no {:?} child", direction, pivot_node.value, direction.opposite());
                // env::panic(ERR_INCONSISTENT_STATE)
            }
        }
    }

    fn push_black(&self, node: &mut RedBlackNode<T>) {
        // println!("TREE: push_black() value={:?}", node.value);
        node.color -= 1;
        let mut left_child_node = self.get_left_child(&node).expect("left child must exist for color change to black");
        let mut right_child_node = self.get_right_child(&node).expect("right child must exist for color change to black");

        left_child_node.color += 1;
        right_child_node.color += 1;

        self.update_node(node);
        self.update_node(&left_child_node);
        self.update_node(&right_child_node);
    }

    fn pull_black(&self, node: &mut RedBlackNode<T>) {
        // println!("TREE: pull_black() value={:?}", node.value);
        if let Some(mut left_child_node) = self.get_left_child(&node) {
            left_child_node.color -= 1;
            self.update_node(&left_child_node);
        }
        
        if let Some(mut right_child_node) = self.get_right_child(&node) {
            right_child_node.color -= 1;
            self.update_node(&right_child_node);
        }
                
        node.color += 1;
        self.update_node(node);
    }

    /// Returns none if the value is not present in the tree,
    /// otherwise removes and returns that value from the tree
    pub fn remove(&mut self, value: &<T as RedBlackNodeValue>::OrdValue) -> Option<T> {
        let removed_value = if let Some(mut u) = self.find_parent(value) {
            // value does not exist in the tree
            if &u.value.ord_value() != &value {
                // println!("TREE: remove() {:?} does not exist in tree", value);
                return None 
            }

            let mut w;
            let child_direction = if let Some(is_w) = self.get_right_child(&u) {
                w = is_w;
                // println!("TREE: remove() right child of u={:?} is w={:?}", value, w.value);

                while let Some(left_child_node) = self.get_left_child(&w) {
                    w = left_child_node;
                }

                // println!("TREE: remove() leftmost child of w is {:?}", w.value);
                
                std::mem::swap(&mut u.value, &mut w.value);
                self.update_node(&u);

                Direction::Right
                // self.get_right_child(&w)
            } else {
                // println!("TREE: remove() right child of u={:?} is w=None", u.value);

                w = u;
                Direction::Left
                // self.get_left_child(&w)
            };

            // println!("TREE: remove() w is {:?} child", w.child_direction());
            // let wp = self.get_parent(&w);
            // println!("TREE: remove() w.parent.left.color = {:?}", wp.as_ref().map(|p| self.get_left_child(p).map_or(1, |n| n.color)));
            // println!("TREE: remove() w.parent.right.color = {:?}", wp.as_ref().map(|p| self.get_right_child(p).map_or(1, |n| n.color)));

            let w_parent = self.splice(&w);

            if let Some(is_u) = self.get_child(&w, &child_direction) {
                u = is_u;
                u.color += w.color;
                // println!("TREE: remove() w = {:?}, u = w.{:?}_child = {:?}, u.color = {:?}", w.value, child_direction, u.value, u.color);
                self.update_node(&u);
                self.remove_fixup(u); 
            } else if w.is_black() {
                // println!("TREE: remove() w = {:?}, u = w.{:?}_child = None, and double black", w.value, child_direction);
                // we have a double black node, but it is a nil node (None)
                if let Some(mut parent_node) = w_parent {

                    let left_child_is_red = self.get_left_child(&parent_node).map_or(false, |n| n.is_red());
                    if left_child_is_red {
                        self.rotate(&Direction::Right, &mut parent_node, true);
                    }

                    assert!(self.get_left_child(&parent_node).map_or(true, |n| n.is_black()));
                    assert!(self.get_right_child(&parent_node).map_or(true, |n| n.is_black()));
                    
                    u = if parent_node.has_left_child() {
                        self.fix_double_black_right_child_node(parent_node)
                    } else {
                        self.fix_double_black_left_child_node(&mut parent_node)
                    };
                    self.remove_fixup(u); 
                }
            } else {
                // no double black node, but need to restore left leaning property
                self.restore_left_leaning_invariant(w_parent);
            }

            Some(w.value)

            // for node in self.iter() {
            //     println!("checking node {:?} for invariant", node);
            // }
            // true
        } else {
            // value does not exist in the tree
            // println!("TREE: remove() {:?} does not exist in tree", value);
            None
        };

        self.update();
        self.cache.clear();
        removed_value
    }

    // Returns parent of the spliced node
    fn splice(&mut self, w: &RedBlackNode<T>) -> Option<RedBlackNode<T>> {
        // println!("TREE: splice(w), w={:?}", w.value);
        let mut s = self
            .get_left_child(w)
            .or_else(|| self.get_right_child(&w));

        // println!("TREE: splice(w), s = {:?}", s.as_ref().map(|n| &n.value));
        let w_parent = self.get_parent(w);

        // println!("TREE: splice(w), w.parent = {:?}", w_parent.as_ref().map(|n| &n.value));
        // Replace node with its child
        let updated_w_parent = if let Some(mut w_parent_node) = w_parent {
            // println!("TREE: splice(w), w.parent.{:?}_child = s = {:?}. s.color={:?}", w.child_direction(), s.as_ref().map(|n| &n.value), s.as_ref().map(|n| n.color));
            w_parent_node.set_child(s.as_mut(), &w.child_direction());
            self.update_node(&w_parent_node);
            Some(w_parent_node)
        } else if let Some(s_node) = s.as_mut() {
            // println!("TREE: splice(w), child s = {:?} is new root!", s_node.value);
            // child node is new root
            s_node.parent_key = None;
            self.root_key = Some(s_node.key().clone());
            None
        } else {
            // child_node (s) is new root and is none
            self.root_key = None;
            None
        };

        // update child node if it exists
        if let Some(s_node) = s {
            self.update_node(&s_node);
        }

        self.delete_node(&w);

        updated_w_parent
    }
    
    fn remove_fixup(&mut self, mut u: RedBlackNode<T>) {
        // println!("TREE: remove_fixup() u={:?}", u.value);
        while u.is_double_black() {
            match self.remove_case(&u) {
                Some((case, u_parent)) => {
                    if let Some(new_node) = self.fix_double_black_node(case, u_parent) {
                        u = new_node;
                    }
                },
                None => { // node is root
                    u.color = 1; // black
                }
            }
            self.update_node(&u);
        }

        // println!("TREE: remove_fixup() done! u={:?}", u.value);
        self.restore_left_leaning_invariant(self.get_parent(&u));
    }

    fn remove_case(&self, u: &RedBlackNode<T>) -> Option<(DoubleBlackNodeCase, RedBlackNode<T>)> {
        use DoubleBlackNodeCase::*;
        self.get_parent(u)
            .map_or(None, |u_parent| {
                if (u.is_left_child() && u.is_red()) || 
                    self.get_left_child(&u_parent)
                        .map_or(false, |left_child_node| left_child_node.is_red()) 
                {
                    Some((LeftSiblingIsRed, u_parent))
                } else if u.is_left_child() {
                    Some((NodeIsLeftChild, u_parent))
                } else {
                    Some((NodeIsRightChild, u_parent))
                }
            })
    }

    fn restore_left_leaning_invariant(&mut self, parent: Option<RedBlackNode<T>>) {
        if let Some(mut w) = parent {
            let right_child_is_red = self.get_right_child(&w).map_or(false, |right_child_node| right_child_node.is_red());
            let left_child_is_black = self.get_left_child(&w).map_or(true, |left_child_node| left_child_node.is_black());

            if right_child_is_red && left_child_is_black {
                // println!("TREE: restore_left_leaning_invariant() parent={:?}", w.value);
                self.rotate(&Direction::Left, &mut w, true);
            }
        }
    }
    
    fn fix_double_black_node(&mut self, case: DoubleBlackNodeCase, mut u_parent: RedBlackNode<T>) -> Option<RedBlackNode<T>> {
        // println!("TREE: fix_double_black_node() case {:?}", case);
        use DoubleBlackNodeCase::*;
        match case {
            LeftSiblingIsRed => {
                self.rotate(&Direction::Right, &mut u_parent, true);
                None
            },
            NodeIsLeftChild => Some(self.fix_double_black_left_child_node(&mut u_parent)),
            NodeIsRightChild => Some(self.fix_double_black_right_child_node(u_parent))
        }
    }
    
    fn fix_double_black_left_child_node(&mut self, w: &mut RedBlackNode<T>) -> RedBlackNode<T> {
        // println!("TREE: remove_fixup_case2() w = {:?}, w.color = {:?}", w.value, w.color);
        self.pull_black(w);
        
        self.rotate(&Direction::Left, w, true);
        
        if self.get_right_child(&w).map_or(false, |r| r.is_red()) {
            // println!("---> w={:?} has parent={:?}, rightchild q={:?}", 
            //     w.value, 
            //     self.get_parent(&w).expect("v").value,
            //     self.get_right_child(&w).expect("q").value
            // );

            // let q_b = self.get_right_child(&w).expect("q");
            // self.assert_left_leaning_invariant(&q_b);
            
            self.rotate(&Direction::Left, w, false);
            // self.assert_left_leaning_invariant(&w);
            
            let intermediate_q = self.get_parent(&w).expect("q");
            let mut v = self.get_parent(&intermediate_q).expect("v");
            // let qq = self.get_left_child(&v).expect("qq");
            // println!("---> w={:?} has parent q={:?}, q parent v={:?}, v lchild q={:?}", 
            //     w.value, 
            //     intermediate_q.value,
            //     v.value,
            //     qq.value
            // );
            // println!("TREE: remove_fixup_case2() w = {:?}. w.color = {:?}", w.value, w.color);
            // println!("TREE: remove_fixup_case2() q = {:?}. q.color = {:?}", intermediate_q.value, intermediate_q.color);
            // println!("TREE: remove_fixup_case2() v = {:?}. v.color = {:?}", v.value, v.color);
            // self.flip_right(v)
            // let mut right_child_node = self.get_right_child(&node).expect("right child must exist");
            self.rotate(&Direction::Right, &mut v, true);
            let mut q = self.get_parent(w).expect("parent must exist");
            // println!("TREE: remove_fixup_case2() w = {:?}. w.color = {:?}", w.value, w.color);
            // println!("TREE: remove_fixup_case2() q = {:?}. q.color = {:?}", q.value, q.color);
            // println!("---> parent of q is {:?}", self.get_parent(&q).map(|p| p.value));
            // println!("TREE: remove_fixup_case2() v = {:?}. v.color = {:?}", v.value, v.color);
            // self.push_black(q)
            self.push_black(&mut q);
            let mut updated_v = self.get_right_child(&q).expect("v exists");

            if self.get_right_child(&updated_v).map_or(false, |n| n.is_red()) {
                self.rotate(&Direction::Left, &mut updated_v, true);
            }

            // q
            self.get_parent(&updated_v).expect("parent must exist")
        } else {
            let v = self.get_parent(w).expect("parent must exist");
            v
        }
    }
    
    fn fix_double_black_right_child_node(&mut self, mut w: RedBlackNode<T>) -> RedBlackNode<T> {
        // println!("TREE: remove_fixup_case3() w = {:?}, w.color = {:?}", w.value, w.color);
        self.pull_black(&mut w);
        self.rotate(&Direction::Right, &mut w, true); // w is now red
        assert!(w.is_red());
        
        if self.get_left_child(&mut w).map_or(false, |l| l.is_red()) {
            // q-w is red-red
            self.rotate(&Direction::Right, &mut w, false);

            let intermediate_q = self.get_parent(&w).expect("q exists");
            let mut v = self.get_parent(&intermediate_q).expect("v exists");
            self.rotate(&Direction::Left, &mut v, true);

            let mut q = self.get_parent(&v).expect("parent exists");
            self.push_black(&mut q);

            return q
        }

        let mut v = self.get_parent(&w).expect("left child must exist");
        assert!(v.is_black());

        if self.get_left_child(&v).map_or(false, |n| n.is_red()) {
            self.push_black(&mut v);
            v
        } else { // ensure left-leaning property
            self.rotate(&Direction::Left, &mut v, true);
            w = self.get_parent(&v).expect("parent exists");
            w
        }
    }
}

struct RedBlackTreeIter<'a, T> {
    tree: &'a RedBlackTree<T>,
    visited: Vec<RedBlackNode<T>>,
    root: Option<RedBlackNode<T>>,
}

struct RedBlackTreeRange<'a, T, R> {
    iter: RedBlackTreeIter<'a, T>,
    init: bool,
    done: bool,
    range: R
}

impl<'a, T, R> RedBlackTreeRange<'a, T, R> 
where
    T: BorshSerialize + BorshDeserialize + RedBlackNodeValue + std::fmt::Debug,
    <T as RedBlackNodeValue>::OrdValue: std::fmt::Debug
{
    pub fn new(tree: &'a RedBlackTree<T>, range: R) -> Self {
        Self {
            iter: RedBlackTreeIter {
                root: None,
                tree,
                visited: vec!()
            },
            init: false,
            done: false,
            range
        }
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
        if self.done {
            return None
        } 

        if !self.init {
            // initialize the root. 
            // for ascending traversals, this will be the start bound
            self.iter.root = match self.range.start_bound() {
                Bound::Included(value) => self.iter.tree.floor(value),
                Bound::Excluded(value) => self.iter.tree.above(value),
                Bound::Unbounded => self.iter.tree.get_root()
            };
            self.init = true;
        }
        
        let node = <RedBlackTreeIter<T> as Iterator>::next(&mut self.iter)
            .filter(|node| self.range.contains(&node.value));
        
        if node.is_none() {
            self.done = true;
        }

        node
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
        if self.done {
            return None
        } 

        if !self.init {
            // initialize the root. 
            // for ascending traversals, this will be the end bound
            self.iter.root = match self.range.end_bound() {
                Bound::Included(value) => self.iter.tree.ceil(value),
                Bound::Excluded(value) => self.iter.tree.below(value),
                Bound::Unbounded => self.iter.tree.get_root()
            };
            self.init = true;
        }
        
        let node = <RedBlackTreeIter<T> as DoubleEndedIterator>::next_back(&mut self.iter)
            .filter(|node| self.range.contains(&node.value));
        
        if node.is_none() {
            self.done = true;
        }

        node
    }
}

impl<'a, T, R> std::iter::FusedIterator for RedBlackTreeRange<'a, T, R> 
where
    T: BorshSerialize + BorshDeserialize + RedBlackNodeValue + std::fmt::Debug,
    <T as RedBlackNodeValue>::OrdValue: std::fmt::Debug,
    R: RangeBounds<T>
{}

impl<'a, T> RedBlackTreeIter<'a, T> 
where
    T: BorshSerialize + BorshDeserialize + RedBlackNodeValue + std::fmt::Debug,
    <T as RedBlackNodeValue>::OrdValue: std::fmt::Debug
{
    fn new(tree: &'a RedBlackTree<T>) -> Self {
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

impl<'a, T> IntoIterator for &'a RedBlackTree<T>
    where
        T: BorshSerialize + BorshDeserialize + RedBlackNodeValue + std::fmt::Debug,
        <T as RedBlackNodeValue>::OrdValue: std::fmt::Debug
{
    type Item = T;
    type IntoIter = Box<dyn DoubleEndedIterator<Item=Self::Item> + 'a>;

    fn into_iter(self) -> Self::IntoIter {
        Box::new(RedBlackTreeIter::new(self).map(|node| node.value))
    }
}

impl<T> Set<T> for RedBlackTree<T> 
where 
    T: BorshSerialize + BorshDeserialize + RedBlackNodeValue + std::fmt::Debug,
    <T as RedBlackNodeValue>::OrdValue: std::fmt::Debug
{

    fn contains(&self, element: &T) -> bool {
        Self::contains(self, element)
    }

    fn remove(&mut self, element: &T) -> bool {
        self.remove(element.ord_value()).is_some()
    }

    fn insert(&mut self, element: &T) -> bool {
        Self::insert(self, element)
    }

    fn clear(&mut self) {
        Self::clear(self);
    }

    fn to_vec(&self) -> std::vec::Vec<T> {
        Self::iter(self).collect()
    }

    fn iter<'a>(&'a self) -> Box<dyn Iterator<Item = T> + 'a> {
        Box::new(self.iter())
    }

    fn extend<IT: IntoIterator<Item = T>>(&mut self, iter: IT) {
        for value in iter {
            self.insert(&value);
        }
    }
}

impl<T> TreeSet<T> for RedBlackTree<T> 
where 
    T: BorshSerialize + BorshDeserialize + RedBlackNodeValue + std::fmt::Debug,
    <T as RedBlackNodeValue>::OrdValue: std::fmt::Debug
{    
    /// Returns the smallest stored value from the tree
    fn min(&self) -> Option<T> {
        self.get_root()
            .map(|root| self.get_leaf_child(&root, &Direction::Left).unwrap_or(root))
            .map(|min| min.value)        
    }
    
    /// Returns the largest stored value from the tree
    fn max(&self) -> Option<T> {
        self.get_root()
            .map(|root| self.get_leaf_child(&root, &Direction::Right).unwrap_or(root))
            .map(|max| max.value)  
    }

    /// Returns the smallest value that is strictly greater than value given as the parameter
    fn above(&self, value: &T) -> Option<T> {
        Self::above(self, value).map(|node| node.value)
    }

    /// Returns the largest value that is strictly less than value given as the parameter
    fn below(&self, value: &T) -> Option<T> {
        Self::below(self, value).map(|node| node.value)
    }

    /// Returns the largest value that is greater or equal to value given as the parameter
    fn ceil(&self, value: &T) -> Option<T> {
        Self::ceil(self, value).map(|node| node.value)
    }
    
    /// Returns the smallest value that is greater or equal to value given as the parameter
    fn floor(&self, value: &T) -> Option<T> {
        Self::floor(self, value).map(|node| node.value)
    }

    /// Iterates through values in ascending order starting at value that is greater than
    /// or equal to the value supplied
    fn iter_from<'a>(&'a self, value: T) -> Box<dyn Iterator<Item = T> + 'a> {
        Box::new(self.range(value..))
    }

    /// Iterates through values in descending order
    fn iter_rev<'a>(&'a self) -> Box<dyn Iterator<Item = T> + 'a> {
        Box::new(self.iter().rev())
    }

    /// Iterates through values in descending order starting at value that is less than
    /// or equal to the value supplied
    fn iter_rev_from<'a>(&'a self, value: T) -> Box<dyn Iterator<Item = T> + 'a> {
        Box::new(self.range(..=value))
    }

    /// Iterate over K values in ascending order
    ///
    /// # Panics
    ///
    /// Panics if range start > end.
    /// Panics if range start == end and both bounds are Excluded.
    fn range<'a>(&'a self, r: (Bound<T>, Bound<T>)) -> Box<dyn Iterator<Item = T> + 'a> {
        Box::new(self.range(r))
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[cfg(test)]
mod test {
    use super::RedBlackTree;
    use crate::{env, MockedBlockchain};
    use crate::collections::set;
    use near_vm_logic::types::AccountId;
    use near_vm_logic::VMContext;
    use rand::{Rng, SeedableRng};
    use std::collections::BTreeSet;

    fn alice() -> AccountId {
        "alice.near".to_string()
    }

    fn bob() -> AccountId {
        "bob.near".to_string()
    }

    fn carol() -> AccountId {
        "carol.near".to_string()
    }

    fn set_env() {
        let context = VMContext {
            current_account_id: alice(),
            signer_account_id: bob(),
            signer_account_pk: vec![0, 1, 2],
            predecessor_account_id: carol(),
            input: vec![],
            block_index: 0,
            block_timestamp: 0,
            account_balance: 0,
            account_locked_balance: 0,
            storage_usage: 10u64.pow(6),
            attached_deposit: 0,
            prepaid_gas: std::u64::MAX,
            random_seed: vec![0, 1, 3, 4, 5],
            is_view: false,
            output_data_receivers: vec![],
            epoch_height: 0,
        };
        let storage = match env::take_blockchain_interface() {
            Some(mut bi) => bi.as_mut_mocked_blockchain().unwrap().take_storage(),
            None => Default::default(),
        };
        env::set_blockchain_interface(Box::new(MockedBlockchain::new(
            context,
            Default::default(),
            Default::default(),
            vec![],
            storage,
        )));
    }

    #[test]
    pub fn test_add() {
        set_env();
        let mut tree = RedBlackTree::default();
        let mut rng = rand_xorshift::XorShiftRng::seed_from_u64(0);
        for _ in 0..500 {
            let value = rng.gen::<u64>();
            tree.insert(&value);
        }
    }

    #[test]
    pub fn test_iter_sorted() {
        set_env();
        let mut tree = RedBlackTree::default();
        let mut rng = rand_xorshift::XorShiftRng::seed_from_u64(4);
        let mut set = BTreeSet::new();
        for _ in 0..500 {
            let value = rng.gen::<u64>();
            set.insert(value);
            tree.insert(&value);
        }

        for val in set.iter().zip(tree.iter()) {
            let (set_value, tree_value) = val;
            assert_eq!(*set_value, tree_value);
        }        
    }

    #[test]
    pub fn test_iter_sorted_with_remove() {
        set_env();
        let mut tree = RedBlackTree::default();
        let mut rng = rand_xorshift::XorShiftRng::seed_from_u64(4);
        let mut set = BTreeSet::new();

        let mut values = vec!();

        for _ in 0..250 {
            let value = rng.gen::<u64>();
            set.insert(value);
            tree.insert(&value);
            if value % 2 == 0 {
                values.push(value);
            }
        }

        for value in values.iter() {
            assert!(set.remove(value));
            assert_eq!(tree.remove(value), Some(*value))
        }

        for val in set.iter().zip(tree.iter()) {
            let (set_value, tree_value) = val;
            assert_eq!(*set_value, tree_value);
        }        

        assert_eq!(set.len(), tree.len as usize)
    }

    #[test]
    pub fn test_iter_desc_sorted() {
        set_env();
        let mut tree = RedBlackTree::default();
        let mut rng = rand_xorshift::XorShiftRng::seed_from_u64(4);
        let mut set = BTreeSet::new();
        for _ in 0..500 {
            let value = rng.gen::<u64>();
            set.insert(value);
            tree.insert(&value);
        }

        for val in set.iter().rev().zip(tree.iter().rev()) {
            let (set_value, tree_value) = val;
            assert_eq!(*set_value, tree_value);
        }        
    }

    #[test]
    pub fn test_iter_desc_sorted_with_remove() {
        set_env();
        let mut tree = RedBlackTree::default();
        let mut rng = rand_xorshift::XorShiftRng::seed_from_u64(4);
        let mut set = BTreeSet::new();

        let mut values = vec!();

        for _ in 0..250 {
            let value = rng.gen::<u64>();
            set.insert(value);
            tree.insert(&value);
            if value % 2 == 0 {
                values.push(value);
            }
        }

        for value in values.iter() {
            assert!(set.remove(value));
            assert_eq!(tree.remove(value), Some(*value))
        }

        for val in set.iter().rev().zip(tree.iter().rev()) {
            let (set_value, tree_value) = val;
            assert_eq!(*set_value, tree_value);
        }        

        assert_eq!(set.len(), tree.len as usize)
    }

    #[test]
    pub fn test_insert() {
        set_env();
        set::tests::test_insert::<RedBlackTree<u64>>()
    }

    #[test]
    pub fn test_insert_remove() {
        set_env();
        set::tests::test_insert_remove::<RedBlackTree<u64>>()
    }

    #[test]
    pub fn test_remove_last_reinsert() {
        set_env();
        set::tests::test_remove_last_reinsert::<RedBlackTree<u64>>()
    }

    #[test]
    pub fn test_insert_override_remove() {
        set_env();
        set::tests::test_insert_override_remove::<RedBlackTree<u64>>()
    }

    #[test]
    pub fn test_contains_non_existent() {
        set_env();
        set::tests::test_contains_non_existent::<RedBlackTree<u64>>()
    }

    #[test]
    pub fn test_to_vec() {
        set_env();
        set::tests::test_to_vec::<RedBlackTree<u64>>()
    }

    #[test]
    pub fn test_clear() {
        set_env();
        set::tests::test_clear::<RedBlackTree<u64>>()
    }

    #[test]
    pub fn test_iter() {
        set_env();
        set::tests::test_iter::<RedBlackTree<u64>>()
    }

    #[test]
    pub fn test_extend() {
        set_env();
        set::tests::test_extend::<RedBlackTree<u64>>()
    }
}