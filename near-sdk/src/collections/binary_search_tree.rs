use crate::collections::next_trie_id;
use crate::env;
use borsh::{BorshDeserialize, BorshSerialize};
use std::marker::PhantomData;
// use std::mem::size_of;

const ERR_INCONSISTENT_STATE: &[u8] = b"The collection is an inconsistent state. Did previous smart contract execution terminate unexpectedly?";
const ERR_ELEMENT_DESERIALIZATION: &[u8] = b"Cannot deserialize element";
const ERR_ELEMENT_SERIALIZATION: &[u8] = b"Cannot serialize element";

type EnvStorageKey = Vec<u8>;

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

impl<T> Ord for RedBlackNode<T> 
where
    T: Ord
{
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.value.cmp(&other.value)
    }
}

impl<T> PartialOrd for RedBlackNode<T> 
where
    T: Ord
{
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.value.cmp(&other.value))
    }
}

impl<T> PartialEq for RedBlackNode<T> 
where
    T: Ord
{
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
    }
}

impl<T> Eq for RedBlackNode<T> 
where
    T: Ord
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
    T: Ord
{

}

#[derive(BorshSerialize, BorshDeserialize)]
pub struct RedBlackTree<T> {
    prefix: EnvStorageKey,
    len: u64,
    root_key: Option<EnvStorageKey>,
    // TODO store indices that have been removed. 
    node_value: PhantomData<T>
}

pub enum Either<T> {
    Parent((DoubleBlackNodeCase, RedBlackNode<T>)),
    Child(RedBlackNode<T>)
}

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

impl<T> RedBlackTree<T> {
    fn new(prefix: EnvStorageKey) -> Self {
        Self {
            prefix, 
            len: 0,
            root_key: None,
            node_value: PhantomData
        }
    }

    fn get_node_raw(&self, key: &EnvStorageKey) -> Option<Vec<u8>> {
        let lookup_key = [&self.prefix, key.as_slice()].concat();
        println!("RAW: looking up node for key {:?}", key);
        env::storage_read(&lookup_key)
    }

    fn insert_node_raw(&self, key: &EnvStorageKey, node: &Vec<u8>) {
        let lookup_key = [&self.prefix, key.as_slice()].concat();
        println!("RAW: inserting node for key {:?}", key);
        if env::storage_write(&lookup_key, node) {
            panic!("insert node raw panic");
            env::panic(ERR_INCONSISTENT_STATE) // Node should not exist already
        }
    }

    fn update_node_raw(&self, key: &EnvStorageKey, node: &Vec<u8>) {
        let lookup_key = [&self.prefix, key.as_slice()].concat();
        println!("RAW: updating node for key {:?}", key);
        if !env::storage_write(&lookup_key, node) { 
            panic!("update node raw panic");
            env::panic(ERR_INCONSISTENT_STATE) // Node should already exist
        }
    }
}

impl<T> Default for RedBlackTree<T> {
    fn default() -> Self {
        Self::new(next_trie_id())
    }
}

impl<T> RedBlackTree<T> 
where 
    T: BorshSerialize + BorshDeserialize + Ord + std::fmt::Debug
{
    fn deserialize_element(raw_element: &[u8]) -> RedBlackNode<T> {
        match RedBlackNode::try_from_slice(&raw_element) {
            Ok(x) => x,
            Err(_) => env::panic(ERR_ELEMENT_DESERIALIZATION),
        }
    }

    fn serialize_element(element: &RedBlackNode<T>) -> Vec<u8> {
        match element.try_to_vec() {
            Ok(x) => x,
            Err(_) => env::panic(ERR_ELEMENT_SERIALIZATION),
        }
    }

    fn get_node(&self, key: &EnvStorageKey) -> Option<RedBlackNode<T>> {
        self.get_node_raw(key).map(|raw_node| Self::deserialize_element(&raw_node))
    }

    fn insert_node(&self, node: &RedBlackNode<T>) {
        let key = node.key();
        self.insert_node_raw(key, &Self::serialize_element(node))
    }

    fn update_node(&self, node: &RedBlackNode<T>) {
        self.update_node_raw(node.key(), &Self::serialize_element(node))
    }

    fn get_root(&self) -> Option<RedBlackNode<T>> {
        self.root_key.as_ref().map(|key| self.get_node(key).expect("root node must exist"))
    }

    fn get_parent(&self, node: &RedBlackNode<T>) -> Option<RedBlackNode<T>> {
        node.parent_key.as_ref().map(|key| self.get_node(key)).flatten()
    }

    pub fn get_left_child(&self, node: &RedBlackNode<T>) -> Option<RedBlackNode<T>> {
        node.left_node_key().map(|key| self.get_node(key)).flatten()
    }

    pub fn get_right_child(&self, node: &RedBlackNode<T>) -> Option<RedBlackNode<T>> {
        node.right_node_key().map(|key| self.get_node(key)).flatten()
    }

    pub fn get_child(&self, node: &RedBlackNode<T>, direction: &Direction) -> Option<RedBlackNode<T>> {
        use Direction::*;
        match direction {
            Left => self.get_left_child(node),
            Right => self.get_right_child(node)
        }
    }

    fn new_node_key(&mut self) -> EnvStorageKey {
        self.len += 1;
        self.len.clone().to_le_bytes().to_vec()
    }

    // 2 writes to env::storage
    fn add_child(&mut self, mut parent: RedBlackNode<T>, child_value: T) -> Option<RedBlackNode<T>> {
        println!("TREE: add_child() parent key {:?}", parent.key());
        use std::cmp::Ordering::*;
        match child_value.cmp(&parent.value) {
            Greater => {
                println!("TREE: add_child() adding right child -->");
                let key = self.new_node_key();
                parent.right_key = Some(key.clone());
                let child_node = RedBlackNode {
                    value: child_value,
                    key,
                    parent_key: Some(parent.key().clone()),
                    is_right_child: true,
                    color: 0, // Red
                    left_key: None,
                    right_key: None
                };
                self.insert_node(&child_node);
                self.update_node(&parent);
                Some(child_node)
            },
            Less => {
                println!("TREE: add_child() adding left child <--");
                let key = self.new_node_key();
                parent.left_key = Some(key.clone());
                let child_node = RedBlackNode {
                    value: child_value,
                    key,
                    parent_key: Some(parent.key().clone()),
                    is_right_child: false,
                    color: 0, // Red
                    left_key: None,
                    right_key: None
                };
                self.insert_node(&child_node);
                self.update_node(&parent);
                Some(child_node)
            },
            Equal => {
                // the value already exists in the tree
                None
            }
        }
    }

    // 1 + O(logN) reads from env::storage
    fn find_parent(&self, child_value: &T) -> Option<RedBlackNode<T>> {
        println!("TREE: find_parent()");
        use std::cmp::Ordering::*;
        let mut root = self.get_root();
        let mut parent = None; 
        
        while let Some(parent_node) = root {
            println!("TREE: find_parent() : {:?} gtlte {:?} = {:?}", child_value, parent_node.value, child_value.cmp(&parent_node.value));
            root = match child_value.cmp(&parent_node.value) {
                Greater => self.get_right_child(&parent_node),
                Less => self.get_left_child(&parent_node),
                Equal => None
            };
            println!("TREE: find_parent() : new root = {:?}", root.as_ref().map(|n| n.key()));
            parent = Some(parent_node);
        }
        println!("TREE: find_parent() : parent of {:?} is {:?}, parent key {:?}", child_value, parent.as_ref().map(|p| &p.value), parent.as_ref().map(|p| p.key()));
        parent
    }

    fn push_black(&self, node: &mut RedBlackNode<T>) {
        println!("TREE: push_black() key {:?}", node.key());
        let mut left_child_node = self.get_left_child(&node).expect("left child must exist for color change to black");
        let mut right_child_node = self.get_right_child(&node).expect("right child must exist for color change to black");
        
        // TODO check if node is already red -- indicates an invariant violation
        node.color -= 1;
        left_child_node.color += 1;
        right_child_node.color += 1;

        self.update_node(node);
        self.update_node(&left_child_node);
        self.update_node(&right_child_node);
    }

    fn pull_black(&self, node: &mut RedBlackNode<T>) {
        println!("TREE: pull_black() key {:?}", node.key());
        if let Some(mut left_child_node) = self.get_left_child(&node) {
            left_child_node.color -= 1;
            self.update_node(&left_child_node);
        }
        
        if let Some(mut right_child_node) = self.get_right_child(&node) {
            right_child_node.color -= 1;
            self.update_node(&right_child_node);
        }
        
        // TODO check if node is already black -- indicates an invariant violation
        
        node.color += 1;

        self.update_node(node);
        // self.update_node(&left_child_node);
        // self.update_node(&right_child_node);
    }

    fn rotate(&mut self, direction: &Direction, pivot_node: &mut RedBlackNode<T>, swap_colors: bool) { // O(1)
        println!("TREE: rotate() {:?} pivot node key {:?}", direction, pivot_node.key());
        use Direction::*;
        match self.get_child(pivot_node, &direction.opposite()) {
            Some(mut child_node) => {
                // Replace pivot node with its right child
                if swap_colors {
                    let parent_color = pivot_node.color;
                    let child_color = child_node.color;
                
                    pivot_node.color = child_color;
                    child_node.color = parent_color;
                }

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
                    println!("TREE: rotate() {:?} grand child updating", direction);
                    self.update_node(&grand_child_node);
                } else {
                    match direction {
                        Left => pivot_node.right_key = None,
                        Right => pivot_node.left_key = None
                    }
                }

                // Replace pivot node's former right child's left child with pivot node
                child_node.set_child(Some(pivot_node), direction);

                println!("TREE: rotate() {:?} pivot node updating", direction);
                self.update_node(pivot_node);
                println!("TREE: rotate() {:?} child node updating", direction);
                self.update_node(&child_node);

                // check if child_node is new root
                if child_node.is_root() {
                    self.root_key = Some(child_node.key().clone())
                }
            },
            None => {
                panic!("rotate {:?} panic. {:?} has no {:?} child", direction, pivot_node.value, direction.opposite());
                env::panic(ERR_INCONSISTENT_STATE)
            }
        }
    }

    fn add_red_node(&mut self, child_node: RedBlackNode<T>) { // O(logN)
        use Direction::*;
        println!("TREE: add_red_node() key {:?}", child_node.key());
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
                    parent_node = self.get_parent(&node).expect("parent must exist"); // FIXME this is not necessarily true
                }
    
                if parent_node.is_black() {
                    break
                }
                
                let mut grand_parent_node = self.get_parent(&parent_node).expect("parent must exist"); // FIXME this is not necessarily true
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

    pub fn add(&mut self, value: T) -> bool {
        let child = match self.find_parent(&value) { // O(logN)
            Some(parent) => self.add_child(parent, value), // O(1)
            None => { // O(1)
                println!("inserting new root");
                // new root
                let key = self.new_node_key();
                let root_node = RedBlackNode {
                    value,
                    key,
                    is_right_child: false, // arbitrary default
                    color: 1, // Black
                    parent_key: None,
                    left_key: None,
                    right_key: None
                };
                self.insert_node(&root_node); 
                self.root_key = Some(root_node.key().clone());
                Some(root_node)
            }
        };

        let added_new_node = child.is_some();

        // O(logN)
        if let Some(child_node) = child {
            // FIXME try to minimize number of updates... need to insert here pre-emptively
            // self.insert_node(&child_node);
            self.add_red_node(child_node); // does nothing if child is already black
        }

        added_new_node
    }

    // Returns parent of the spliced node
    pub fn splice(&mut self, node: &RedBlackNode<T>) -> Option<RedBlackNode<T>> {
        let mut child = self
            .get_left_child(node)
            .or_else(|| self.get_right_child(&node));

        let mut parent = self.get_parent(node);

        // Replace node with its child
        if let Some(parent_node) = parent.as_mut() {
            parent_node.set_child(child.as_mut(), &node.child_direction());
            self.update_node(parent_node);
        } else if let Some(child_node) = child.as_mut() {
            // child node is new root
            child_node.parent_key = None;
            self.root_key = Some(child_node.key().clone());
            // FIXME what if child_node is new root and is None??
        }

        // update child node if it exists
        if let Some(child_node) = child {
            self.update_node(&child_node);
        }

        // TODO add call to remove node from env::storage
        self.len -= 1; // TODO check for underflow?
        parent
    }

    pub fn remove(&mut self, value: &T) -> bool {
        // u = self._find_last(x)
        if let Some(mut node) = self.find_parent(value) {
            // if u == self.nil or u.x != x:
            if &node.value != value {
                println!("TREE: remove() {:?} does not exist in tree", value);
                //     return False
                // value does not exist in the tree
                return false 
            }

            // w = u.right
            let mut child_node;
            if let Some(right_child_node) = self.get_right_child(&node) {
                println!("TREE: remove() right child of u={:?} is w={:?}", value, right_child_node.value);
                // else:
                //     while w.left != self.nil:
                //         w = w.left
                child_node = right_child_node;
                while let Some(left_child_node) = self.get_left_child(&child_node) {
                    child_node = left_child_node;
                }

                println!("TREE: remove() leftmost child of w is {:?}", child_node.value);
                
                let right_grand_child = self.get_right_child(&child_node);
                let child_node_child_direction = child_node.child_direction();
                let child_node_color = child_node.color;

                //     u.x = w.x
                std::mem::swap(&mut node.value, &mut child_node.value);
                self.update_node(&node);

                let parent = self.splice(&child_node);

                if let Some(right_grand_child_node) = right_grand_child {
                    node = right_grand_child_node;
                    node.color += child_node_color;
                    self.update_node(&node);
                    self.remove_fixup(node); 
                } else if child_node_color == 1 {
                    // we have a double black node, but it is a nil node (None)
                    if let Some(mut parent_node) = parent {
                        node = if parent_node.has_left_child() {
                            self.fix_double_black_right_child_node(parent_node)
                        } else {
                            self.fix_double_black_left_child_node(&mut parent_node)
                        };
                        // node = self.fix_double_black_right_child_node(parent_node);
                        // node = self.fix_double_black_left_child_node(&mut parent_node);
                        self.remove_fixup(node); 
                    }
                } else {
                    // no double black node, but need to restore left leaning property
                    self.restore_left_leaning(parent);
                }

                //     u = w.right
                // node = right_grand_child_node;

                // node.color += child_node_color;
                
                // done during splicing
                // if let Some(mut parent_node) = self.get_parent(&node) {
                //     parent_node.set_child(Some(&mut node), &child_node_child_direction);
                //     self.update_node(&parent_node);
                // } else {
                //     node.parent_key = None;
                // }
            } else {
                println!("TREE: remove() right child of {:?} is None", value);
                // if w == self.nil:
                //     w = u
                //     u = w.left
                child_node = node;
                // println!("TREE: remove() w=u; {:?} -> {:?}", child_node.value);

                let parent = self.splice(&child_node);

                if let Some(left_grand_child_node) = self.get_left_child(&child_node) {
                    println!("TREE: remove() left child of {:?} is {:?}", child_node.value, left_grand_child_node.value);
                    node = left_grand_child_node;
                    node.color += child_node.color;
                    self.update_node(&node);
                    self.remove_fixup(node); 
                } else if child_node.is_black() {
                    println!("TREE: remove() left child of {:?} is None, and double black", child_node.value);
                    // we have a double black node, but it is a nil node (None)
                    if let Some(mut parent_node) = parent {
                        node = if parent_node.has_left_child() {
                            self.fix_double_black_right_child_node(parent_node)
                        } else {
                            self.fix_double_black_left_child_node(&mut parent_node)
                        };
                        // node = self.fix_double_black_right_child_node(parent_node);
                        // node = self.fix_double_black_left_child_node(&mut parent_node);
                        self.remove_fixup(node); 
                    }
                } else {
                    // no double black node, but need to restore left leaning property
                    self.restore_left_leaning(parent);
                }

                // done during splicing
                // if let Some(mut parent_node) = self.get_parent(&node) {
                //     parent_node.set_child(Some(&mut node), &child_node.child_direction());
                //     self.update_node(&parent_node);
                // } else {
                //     node.parent_key = None;
                // }
            }

            // self.splice(w)
            // u.colour += w.colour
            // u.parent = w.parent 
            // self.remove_fixup(u)
            true
        } else {
            println!("TREE: remove() {:?} does not exist in tree", value);
            // if u == self.nil or u.x != x:
            //     return False
            // value does not exist in the tree
            false
        }
    }

    fn remove_case(&self, node: &RedBlackNode<T>) -> Option<(DoubleBlackNodeCase, RedBlackNode<T>)> {
        use DoubleBlackNodeCase::*;
        self.get_parent(node)
            .map_or(None, |parent_node| {
                if (node.is_left_child() && node.is_red()) || 
                    self.get_left_child(&parent_node)
                        .map_or(false, |left_child_node| left_child_node.is_red()) 
                {
                    Some((LeftSiblingIsRed, parent_node))
                } else if node.is_left_child() {
                    Some((NodeIsLeftChild, parent_node))
                } else {
                    Some((NodeIsRightChild, parent_node))
                }
            })
    }

    fn restore_left_leaning(&mut self, parent: Option<RedBlackNode<T>>) {
        if let Some(mut parent_node) = parent {
            let right_chlid_is_red = self.get_right_child(&parent_node).map_or(false, |right_child_node| right_child_node.is_red());
            let left_child_is_black = self.get_left_child(&parent_node).map_or(true, |left_child_node| left_child_node.is_black());
            // if w.right.colour == red and w.left.colour == black:
            if right_chlid_is_red && left_child_is_black {
                println!("TREE: restore_left_leaning()");
                // self.flip_left(w)
                self.rotate(&Direction::Left, &mut parent_node, true);
            }
        }
    }
    
    fn remove_fixup(&mut self, mut node: RedBlackNode<T>) {
        // while u.colour > black:
        while node.is_double_black() {
            match self.remove_case(&node) {
                Some((case, parent_node)) => {
                    if let Some(new_node) = self.fix_double_black_node(case, parent_node) {
                        node = new_node;
                    }
                },
                None => { // node is root
                    node.color = 1; // black
                    self.update_node(&node);
                }
            }
            /*
            println!("TREE: remove_fixup() {:?} is double black", node.value);
            if let Some(mut parent_node) = self.get_parent(&node) {

            } 
            // if u == self.r:  
            else {
                println!("TREE: remove_fixup() {:?} is new root. coloring black", node.value);
                //     u.colour = black
                node.color = 1; // black
                self.update_node(&node);
            }
            */
        }

        self.restore_left_leaning(self.get_parent(&node));
        
        // if u != self.r:   # restore left-leaning property, if needed
        // if let Some(mut parent_node) = self.get_parent(&node) {
            // println!("TREE: remove_fixup() restoring left leaning property for {:?}", node.value);
            // self.restore_left_leaning(&mut parent_node);
            // w = u.parent
            /*
            let right_chlid_is_red = self.get_right_child(&parent_node).map_or(false, |right_child_node| right_child_node.is_red());
            let left_child_is_black = self.get_left_child(&parent_node).map_or(true, |left_child_node| left_child_node.is_black());
            // if w.right.colour == red and w.left.colour == black:
            if right_chlid_is_red && left_child_is_black {
                // self.flip_left(w)
                self.rotate(&Direction::Left, &mut parent_node, true);
            }
            */
        // }
    }
    
    fn fix_double_black_node(&mut self, case: DoubleBlackNodeCase, mut parent_node: RedBlackNode<T>) -> Option<RedBlackNode<T>> {
        println!("TREE: fix_double_black_node() case {:?}", case);
        use DoubleBlackNodeCase::*;
        match case {
            LeftSiblingIsRed => {
                // println!("TREE: remove_fixup() case 1 for {:?}", node.value);
                self.rotate(&Direction::Right, &mut parent_node, true);
                None
            },
            NodeIsLeftChild => Some(self.fix_double_black_left_child_node(&mut parent_node)),
            NodeIsRightChild => Some(self.fix_double_black_right_child_node(parent_node))
        }
        /*
        // elif u.parent.left.colour == red:
        if (node.is_left_child() && node.is_red()) || 
            self.get_left_child(&parent_node)
                .map_or(false, |left_child_node| left_child_node.is_red()) 
        {
            println!("TREE: remove_fixup() case 1 for {:?}", node.value);
            // u = self.remove_fixup_case1(u)
            self.rotate(&Direction::Right, &mut parent_node, true);
        } 
        // elif u == u.parent.left:
        else if node.is_left_child() {
            println!("TREE: remove_fixup() case 2 for {:?}", node.value);
            node = self.remove_fixup_case2(&mut parent_node);
        }
        // node is right child
        else {
            println!("TREE: remove_fixup() case 3 for {:?}", node.value);
            node = self.remove_fixup_case3(parent_node);
        }
        node
        */
    }
    
    fn fix_double_black_left_child_node(&mut self, node: &mut RedBlackNode<T>) -> RedBlackNode<T> {
        println!("TREE: remove_fixup_case2() w = {:?}", node.value);
        // w = u.parent
        // v = w.right
        let mut right_child_node = self.get_right_child(&node).expect("right child must exist");
        // self.pull_black(w)
        self.pull_black(node);
        // self.flip_left(w)
        self.rotate(&Direction::Left, node, true);
        // q = w.right
        if let Some(mut new_right_child_node) = self.get_right_child(&node) {
            // if q.colour == red:
            if new_right_child_node.is_red() {
            //     self.rotate_left(w)
                self.rotate(&Direction::Left, node, false);
            //     self.flip_right(v)
                self.rotate(&Direction::Right, &mut right_child_node, true);
            //     self.push_black(q)
                self.push_black(&mut new_right_child_node);
            //     if v.right.colour == red:
                if self.get_right_child(&right_child_node).map_or(false, |n| n.is_red()) {
            //         self.flip_left(v)
                    self.rotate(&Direction::Left, &mut right_child_node, true);
                }
            //     return q
                return new_right_child_node
            }
        }
        // else:
        //     return v
        right_child_node
    }
    
    fn fix_double_black_right_child_node(&mut self, mut node: RedBlackNode<T>) -> RedBlackNode<T> {
        println!("TREE: remove_fixup_case3() w = {:?}", node.value);
        // w = u.parent
        // v = w.left
        // let mut left_child_node = self.get_left_child(&node).expect("left child must exist");
        // self.pull_black(w)
        self.pull_black(&mut node);
        // self.flip_right(w)  # w is now red
        self.rotate(&Direction::Right, &mut node, true);
        let mut left_child_node = self.get_parent(&node).expect("left child must exist");
        // q = w.left
        if let Some(mut new_left_child_node) = self.get_left_child(&mut node) {
            // if q.colour == red:  # q-w is red-red
            if new_left_child_node.is_red() {
            //     self.rotate_right(w)
                self.rotate(&Direction::Right, &mut node, false);
            //     self.flip_left(v)
                self.rotate(&Direction::Left, &mut left_child_node, true);
            //     self.push_black(q)
                self.push_black(&mut new_left_child_node);
            //     return q
                return new_left_child_node
            }
        }

        // else:
        //     if v.left.colour == red:
        if self.get_left_child(&left_child_node).map_or(false, |n| n.is_red()) {
        //         self.push_black(v)
            self.push_black(&mut left_child_node);
        //         return v
            left_child_node
        } 
        //     else:  # ensure left-leaning
        else {
        //         self.flip_left(v)
            self.rotate(&Direction::Left, &mut left_child_node, true);
        //         return w
            node
        }
    }

    pub fn iter<'a>(&'a self) -> impl Iterator<Item=T> + 'a {
        RedBlackTreeIter::new(self)
    }
}

struct RedBlackTreeIter<'a, T> {
    tree: &'a RedBlackTree<T>,
    visited: Vec<RedBlackNode<T>>,
    index: u64
}

impl<'a, T> RedBlackTreeIter<'a, T> 
where
    T: BorshSerialize + BorshDeserialize + Ord + std::fmt::Debug
{
    fn new(tree: &'a RedBlackTree<T>) -> Self {
        Self {
            tree,
            visited: vec!(),
            index: 0
        }
    }

    fn get_left_most_child(&mut self, node: RedBlackNode<T>) -> RedBlackNode<T> {
        let mut last_node = node;

        while let Some(left_child_node) = self.tree.get_left_child(&last_node) {
            self.visited.push(last_node); 
            last_node = left_child_node;
        }

        last_node
    }

    fn get_next_node(&mut self, node: RedBlackNode<T>) -> RedBlackNode<T> {
        if node.has_left_child() {
            self.get_left_most_child(node)
        } else {
            node
        }
    }
}

impl<'a, T> Iterator for RedBlackTreeIter<'a, T> 
where
    T: BorshSerialize + BorshDeserialize + Ord + std::fmt::Debug
{
    type Item = T;

    /// in order traversal in ascending order
    /// TODO implement descending
    fn next(&mut self) -> Option<Self::Item> {
        let next_node = self.visited.pop();
        
        // if tree is of length zero, or there are no nodes left return None
        if next_node.is_none() && self.index == self.tree.len {
            self.index = 0; // allows iterator to be re-used
            None
        } 
        // if there is some node that has been visited, visit its right child's left subtree. 
        // then return the visited node
        else if let Some(node) = next_node {
            if let Some(right_child_node) = self.tree.get_right_child(&node) {
                let n = self.get_next_node(right_child_node);
                self.visited.push(n);
            }
            self.index += 1;
            Some(node.value)
        }
        // if this is the first call to next(), visit the left subtree of the root of the tree.
        // then return the leftmost child (leaf node) of the tree 
        else if self.index == 0 {
            let root_node = self.tree.get_root().expect("root must exist");
            let node = self.get_next_node(root_node);
            self.index += 1;
            Some(node.value)
        } 
        // this case implies that there is no next node, in spite of being in the middle of iterating
        else {
            panic!("send inconsistent state error")
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[cfg(test)]
mod test {
    use super::RedBlackTree;
    use crate::{env, MockedBlockchain};
    use near_vm_logic::types::AccountId;
    use near_vm_logic::VMContext;
    use rand::{Rng, SeedableRng};

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
            prepaid_gas: 10u64.pow(18),
            random_seed: vec![0, 1, 2],
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
        let mut rng = rand_xorshift::XorShiftRng::seed_from_u64(0);
        let mut tree = RedBlackTree::default();
        let mut baseline = std::collections::BTreeSet::new();

        let mut remove_these = vec!(12);
        // for x in vec![10u64, 6, 3, 4, 5].into_iter() {
        for i in 0..100 {
            let x = rng.gen::<u64>();
            // if i % 10 == 0 {
            //     remove_these.push(i.clone());
            // }
            println!("TEST: -------- inserting value {:?} ---------", i);
            tree.add(i);
            baseline.insert(i);
        }

        for r in remove_these.into_iter() {
            println!("TEST: -------- removing value {:?} ---------", r);
            assert!(tree.remove(&r));
            assert!(baseline.remove(&r));
        }

        // let actual = tree.to_vec();
        // assert_eq!(tree, baseline);
        // for _ in 0..1001 {
        //     assert_eq!(baseline.pop(), tree.pop());
        // }
        
        // see that iterating through the values leads to identical output
        let iter_thing = baseline.iter().zip(tree.iter());
        // let mut iter_thing = tree.iter();
        for val in iter_thing {
            let (baseline_value, tree_value) = val;
            println!("{:?} vs {:?}", baseline_value, tree_value);
            assert_eq!(*baseline_value, tree_value);
            // let v = iter_thing.next();
            // println!("{:?}", v);
        }

        //let iter_thing_post_remove = baseline.iter().zip(tree.iter());

    }
    #[test]
    fn test_as_set() {

    }
}