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
    is_red: bool,
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

    pub fn is_black(&self) -> bool {
        !self.is_red
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

    pub fn set_right_child(&mut self, node: &mut RedBlackNode<T>) {
        self.right_key = Some(node.key().clone());
        node.parent_key = Some(self.key().clone());
        node.is_right_child = true;
    }

    pub fn set_left_child(&mut self, node: &mut RedBlackNode<T>) {
        self.left_key = Some(node.key().clone());
        node.parent_key = Some(self.key().clone());
        node.is_right_child = false;
    }

    pub fn set_child(&mut self, node: &mut RedBlackNode<T>, direction: &Direction) {
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
    prefix: Vec<u8>,
    len: u64,
    root_key: Option<EnvStorageKey>,
    // TODO store indices that have been removed. 
    node_value: PhantomData<T>
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
    fn new(prefix: Vec<u8>) -> Self {
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
                    is_red: true,
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
                    is_red: true,
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

    fn update_node_to_red(&self, node: &mut RedBlackNode<T>) {
        println!("TREE: update_node_to_red() key {:?}", node.key());
        let mut left_child_node = self.get_left_child(&node).expect("left child must exist for color change to black");
        let mut right_child_node = self.get_right_child(&node).expect("right child must exist for color change to black");
        
        // TODO check if node is already red -- indicates an invariant violation
        node.is_red = true;
        left_child_node.is_red = false;
        right_child_node.is_red = false;

        self.update_node(node);
        self.update_node(&left_child_node);
        self.update_node(&right_child_node);
    }

    fn rotate(&mut self, direction: &Direction, pivot_node: &mut RedBlackNode<T>, swap_colors: bool) { // O(1)
        println!("TREE: rotate_left() pivot node key {:?}", pivot_node.key());
        use Direction::*;
        match self.get_child(pivot_node, &direction.opposite()) {
            Some(mut child_node) => {
                // Replace pivot node with its right child
                if swap_colors {
                    let parent_color = pivot_node.is_red;
                    let child_color = child_node.is_red;
                
                    pivot_node.is_red = child_color;
                    child_node.is_red = parent_color;
                }

                if let Some(mut pivot_parent_node) = self.get_parent(&pivot_node) {
                    pivot_parent_node.set_child(&mut child_node, &pivot_node.child_direction());
                    self.update_node(&pivot_parent_node);
                } else {
                    // pivot node parent_key is none -- assert this! FIXME
                    child_node.parent_key = pivot_node.parent_key.clone();
                    child_node.is_right_child = pivot_node.is_right_child;
                }
                
                // Replace pivot node's right child with former right child's left child
                if let Some(mut grand_child_node) = self.get_child(&child_node, direction) {
                    pivot_node.set_child(&mut grand_child_node, &direction.opposite());
                    println!("TREE: rotate() {:?} grand child updating", direction);
                    self.update_node(&grand_child_node);
                } else {
                    match direction {
                        Left => pivot_node.right_key = None,
                        Right => pivot_node.left_key = None
                    }
                }

                // Replace pivot node's former right child's left child with pivot node
                child_node.set_child(pivot_node, direction);

                println!("TREE: rotate() {:?} pivot node updating", direction);
                self.update_node(pivot_node);
                println!("TREE: rotate() {:?} child node updating", direction);
                self.update_node(&child_node);

                // check if child_node is new root
                if child_node.parent_key.is_none() {
                    self.root_key = Some(child_node.key().clone())
                }
            },
            None => {
                panic!("rotate {:?} panic", direction);
                env::panic(ERR_INCONSISTENT_STATE)
            }
        }
    }

    fn rotate_right(&mut self, pivot_node: &mut RedBlackNode<T>, swap_colors: bool) { // O(1)
        println!("TREE: rotate_right() pivot node key {:?}", pivot_node.key());
        match self.get_left_child(pivot_node) {
            Some(mut left_child_node) => {
                // Replace pivot node with its left child
                if swap_colors {
                    let parent_color = pivot_node.is_red;
                    let child_color = left_child_node.is_red;
                
                    pivot_node.is_red = child_color;
                    left_child_node.is_red = parent_color;
                }

                if let Some(mut pivot_parent_node) = self.get_parent(&pivot_node) {
                    if pivot_node.is_right_child() {
                        pivot_parent_node.set_right_child(&mut left_child_node);
                    } else {
                        pivot_parent_node.set_left_child(&mut left_child_node);
                    }
                    self.update_node(&pivot_parent_node);
                } else {
                    // pivot node parent_key is none -- assert this! FIXME
                    left_child_node.parent_key = pivot_node.parent_key.clone();
                    left_child_node.is_right_child = pivot_node.is_right_child;
                }
                
                // Replace pivot node's left child with former left child's right child
                if let Some(mut right_of_left_child_node) = self.get_right_child(&left_child_node) {
                    pivot_node.set_left_child(&mut right_of_left_child_node);
                    println!("TREE: rotate_right() right of left child updating");
                    self.update_node(&right_of_left_child_node);
                } else {
                    pivot_node.left_key = None;
                }

                // Replace pivot node's former left child's right child with pivot node
                left_child_node.set_right_child(pivot_node);

                println!("TREE: rotate_right() pivot node updating");
                self.update_node(pivot_node);
                println!("TREE: rotate_right() left child updating");
                self.update_node(&left_child_node);
                
                // check if left_child_node is new root
                if left_child_node.parent_key.is_none() {
                    self.root_key = Some(left_child_node.key().clone())
                }
            },
            None => {
                panic!("rotate right panic");
                env::panic(ERR_INCONSISTENT_STATE)
            }
        }
    }

    // fn swap_colors(&self, node: &mut RedBlackNode<T>, child_node: &mut RedBlackNode<T>) {
    //     let parent_color = node.is_red;
    //     let child_color = child_node.is_red;
       
    //     node.is_red = child_color;
    //     child_node.is_red = parent_color;

    //     self.update_node(&node);
    //     self.update_node(&child_node);
    // }

    // fn flip_left(&self, node: &mut RedBlackNode<T>) {
    //     self.swap_colors(node, child_node)
    // }

    fn add_red_node(&mut self, child_node: RedBlackNode<T>) { // O(logN)
        use Direction::*;
        println!("TREE: add_red_node() key {:?}", child_node.key());
        let mut node = child_node;
        while node.is_red {
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
    
                self.update_node_to_red(&mut grand_parent_node);
    
                node = grand_parent_node;
            } else {
                // This node is the root node, color it black
                node.is_red = false;
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
                    is_red: false,
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
        // for x in vec![10u64, 6, 3, 4, 5].into_iter() {
        for _ in 0..100 {
            let x = rng.gen::<u64>();
            println!("TEST: -------- inserting value {:?} ---------", x);
            tree.add(x);
            baseline.insert(x);
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
    }
    #[test]
    fn test_as_set() {

    }
}