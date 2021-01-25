//! Legacy `TreeMap` implementation that is using `UnorderedMap`.
//! DEPRECATED. This implementation is deprecated and may be removed in the future.

use borsh::{BorshDeserialize, BorshSerialize};
use std::ops::Bound;

use crate::collections::UnorderedMap;
use crate::collections::{append, Vector};

/// TreeMap based on AVL-tree
///
/// Runtime complexity (worst case):
/// - `get`/`contains_key`:     O(1) - UnorderedMap lookup
/// - `insert`/`remove`:        O(log(N))
/// - `min`/`max`:              O(log(N))
/// - `above`/`below`:          O(log(N))
/// - `range` of K elements:    O(Klog(N))
///
#[derive(BorshSerialize, BorshDeserialize)]
pub struct LegacyTreeMap<K, V> {
    root: u64,
    val: UnorderedMap<K, V>,
    tree: Vector<Node<K>>,
}

#[derive(Clone, BorshSerialize, BorshDeserialize)]
pub struct Node<K> {
    id: u64,
    key: K,           // key stored in a node
    lft: Option<u64>, // left link of a node
    rgt: Option<u64>, // right link of a node
    ht: u64,          // height of a subtree at a node
}

impl<K> Node<K>
where
    K: Ord + Clone + BorshSerialize + BorshDeserialize,
{
    fn of(id: u64, key: K) -> Self {
        Self { id, key, lft: None, rgt: None, ht: 1 }
    }
}

impl<K, V> LegacyTreeMap<K, V>
where
    K: Ord + Clone + BorshSerialize + BorshDeserialize,
    V: BorshSerialize + BorshDeserialize,
{
    pub fn new(id: Vec<u8>) -> Self {
        Self {
            root: 0,
            val: UnorderedMap::new(append(&id, b'v')),
            tree: Vector::new(append(&id, b'n')),
        }
    }

    pub fn len(&self) -> u64 {
        self.tree.len() as u64
    }

    pub fn clear(&mut self) {
        self.root = 0;
        self.val.clear();
        self.tree.clear();
    }

    fn node(&self, id: u64) -> Option<Node<K>> {
        self.tree.get(id)
    }

    fn save(&mut self, node: &Node<K>) {
        if node.id < self.len() {
            self.tree.replace(node.id, node);
        } else {
            self.tree.push(node);
        }
    }

    pub fn contains_key(&self, key: &K) -> bool {
        self.val.get(key).is_some()
    }

    pub fn get(&self, key: &K) -> Option<V> {
        self.val.get(key)
    }

    pub fn insert(&mut self, key: &K, val: &V) -> Option<V> {
        if !self.contains_key(&key) {
            self.root = self.insert_at(self.root, self.len(), &key);
        }
        self.val.insert(&key, &val)
    }

    pub fn remove(&mut self, key: &K) -> Option<V> {
        if self.contains_key(&key) {
            self.root = self.do_remove(&key);
            self.val.remove(&key)
        } else {
            // no such key, nothing to do
            None
        }
    }

    /// Returns the smallest stored key from the tree
    pub fn min(&self) -> Option<K> {
        self.min_at(self.root, self.root).map(|(n, _)| n.key)
    }

    /// Returns the largest stored key from the tree
    pub fn max(&self) -> Option<K> {
        self.max_at(self.root, self.root).map(|(n, _)| n.key)
    }

    /// Returns the smallest key that is strictly greater than key given as the parameter
    pub fn higher(&self, key: &K) -> Option<K> {
        self.above_at(self.root, key)
    }

    /// Returns the largest key that is strictly less than key given as the parameter
    pub fn lower(&self, key: &K) -> Option<K> {
        self.below_at(self.root, key)
    }

    /// Returns the smallest key that is greater or equal to key given as the parameter
    pub fn ceil_key(&self, key: &K) -> Option<K> {
        if self.contains_key(key) {
            Some(key.clone())
        } else {
            self.higher(key)
        }
    }

    /// Returns the largest key that is less or equal to key given as the parameter
    pub fn floor_key(&self, key: &K) -> Option<K> {
        if self.contains_key(key) {
            Some(key.clone())
        } else {
            self.lower(key)
        }
    }

    /// Iterate all entries in ascending order: min to max, both inclusive
    pub fn iter<'a>(&'a self) -> impl Iterator<Item = (K, V)> + 'a {
        Cursor::asc(&self).into_iter()
    }

    /// Iterate entries in ascending order: given key (exclusive) to max (inclusive)
    pub fn iter_from<'a>(&'a self, key: K) -> impl Iterator<Item = (K, V)> + 'a {
        Cursor::asc_from(&self, key).into_iter()
    }

    /// Iterate all entries in descending order: max to min, both inclusive
    pub fn iter_rev<'a>(&'a self) -> impl Iterator<Item = (K, V)> + 'a {
        Cursor::desc(&self).into_iter()
    }

    /// Iterate entries in descending order: given key (exclusive) to min (inclusive)
    pub fn iter_rev_from<'a>(&'a self, key: K) -> impl Iterator<Item = (K, V)> + 'a {
        Cursor::desc_from(&self, key).into_iter()
    }

    /// Iterate entries in ascending order according to specified bounds.
    ///
    /// # Panics
    ///
    /// Panics if range start > end.
    /// Panics if range start == end and both bounds are Excluded.
    pub fn range<'a>(&'a self, r: (Bound<K>, Bound<K>)) -> impl Iterator<Item = (K, V)> + 'a {
        let (lo, hi) = match r {
            (Bound::Included(a), Bound::Included(b)) if a > b => panic!("Invalid range."),
            (Bound::Excluded(a), Bound::Included(b)) if a > b => panic!("Invalid range."),
            (Bound::Included(a), Bound::Excluded(b)) if a > b => panic!("Invalid range."),
            (Bound::Excluded(a), Bound::Excluded(b)) if a == b => panic!("Invalid range."),
            (lo, hi) => (lo, hi),
        };

        Cursor::range(&self, lo, hi).into_iter()
    }

    pub fn to_vec(&self) -> Vec<(K, V)> {
        self.iter().collect()
    }

    //
    // Internal utilities
    //

    /// Returns (node, parent node) of left-most lower (min) node starting from given node `at`.
    /// As min_at only traverses the tree down, if a node `at` is the minimum node in a subtree,
    /// its parent must be explicitly provided in advance.
    fn min_at(&self, mut at: u64, p: u64) -> Option<(Node<K>, Node<K>)> {
        let mut parent: Option<Node<K>> = self.node(p);
        loop {
            let node = self.node(at);
            match node.clone().and_then(|n| n.lft) {
                Some(lft) => {
                    at = lft;
                    parent = node;
                }
                None => {
                    return node.and_then(|n| parent.map(|p| (n, p)));
                }
            }
        }
    }

    /// Returns (node, parent node) of right-most lower (max) node starting from given node `at`.
    /// As min_at only traverses the tree down, if a node `at` is the minimum node in a subtree,
    /// its parent must be explicitly provided in advance.
    fn max_at(&self, mut at: u64, p: u64) -> Option<(Node<K>, Node<K>)> {
        let mut parent: Option<Node<K>> = self.node(p);
        loop {
            let node = self.node(at);
            match node.clone().and_then(|n| n.rgt) {
                Some(rgt) => {
                    parent = node;
                    at = rgt;
                }
                None => {
                    return node.and_then(|n| parent.map(|p| (n, p)));
                }
            }
        }
    }

    fn above_at(&self, mut at: u64, key: &K) -> Option<K> {
        let mut seen: Option<K> = None;
        loop {
            let node = self.node(at);
            match node.clone().map(|n| n.key) {
                Some(k) => {
                    if k.le(key) {
                        match node.and_then(|n| n.rgt) {
                            Some(rgt) => at = rgt,
                            None => break,
                        }
                    } else {
                        seen = Some(k);
                        match node.and_then(|n| n.lft) {
                            Some(lft) => at = lft,
                            None => break,
                        }
                    }
                }
                None => break,
            }
        }
        seen
    }

    fn below_at(&self, mut at: u64, key: &K) -> Option<K> {
        let mut seen: Option<K> = None;
        loop {
            let node = self.node(at);
            match node.clone().map(|n| n.key) {
                Some(k) => {
                    if k.lt(key) {
                        seen = Some(k);
                        match node.and_then(|n| n.rgt) {
                            Some(rgt) => at = rgt,
                            None => break,
                        }
                    } else {
                        match node.and_then(|n| n.lft) {
                            Some(lft) => at = lft,
                            None => break,
                        }
                    }
                }
                None => break,
            }
        }
        seen
    }

    fn insert_at(&mut self, at: u64, id: u64, key: &K) -> u64 {
        match self.node(at) {
            None => {
                self.save(&Node::of(id, key.clone()));
                at
            }
            Some(mut node) => {
                if key.eq(&node.key) {
                    at
                } else {
                    if key.lt(&node.key) {
                        let idx = match node.lft {
                            Some(lft) => self.insert_at(lft, id, key),
                            None => self.insert_at(id, id, key),
                        };
                        node.lft = Some(idx);
                    } else {
                        let idx = match node.rgt {
                            Some(rgt) => self.insert_at(rgt, id, key),
                            None => self.insert_at(id, id, key),
                        };
                        node.rgt = Some(idx);
                    };

                    self.update_height(&mut node);
                    self.enforce_balance(&mut node)
                }
            }
        }
    }

    // Calculate and save the height of a subtree at node `at`:
    // height[at] = 1 + max(height[at.L], height[at.R])
    fn update_height(&mut self, node: &mut Node<K>) {
        let lft = node.lft.and_then(|id| self.node(id).map(|n| n.ht)).unwrap_or_default();
        let rgt = node.rgt.and_then(|id| self.node(id).map(|n| n.ht)).unwrap_or_default();

        node.ht = 1 + std::cmp::max(lft, rgt);
        self.save(&node);
    }

    // Balance = difference in heights between left and right subtrees at given node.
    fn get_balance(&self, node: &Node<K>) -> i64 {
        let lht = node.lft.and_then(|id| self.node(id).map(|n| n.ht)).unwrap_or_default();
        let rht = node.rgt.and_then(|id| self.node(id).map(|n| n.ht)).unwrap_or_default();

        lht as i64 - rht as i64
    }

    // Left rotation of an AVL subtree with at node `at`.
    // New root of subtree is returned, caller is responsible for updating proper link from parent.
    fn rotate_left(&mut self, node: &mut Node<K>) -> u64 {
        let mut lft = node.lft.and_then(|id| self.node(id)).unwrap();
        let lft_rgt = lft.rgt;

        // at.L = at.L.R
        node.lft = lft_rgt;

        // at.L.R = at
        lft.rgt = Some(node.id);

        // at = at.L
        self.update_height(node);
        self.update_height(&mut lft);

        lft.id
    }

    // Right rotation of an AVL subtree at node in `at`.
    // New root of subtree is returned, caller is responsible for updating proper link from parent.
    fn rotate_right(&mut self, node: &mut Node<K>) -> u64 {
        let mut rgt = node.rgt.and_then(|id| self.node(id)).unwrap();
        let rgt_lft = rgt.lft;

        // at.R = at.R.L
        node.rgt = rgt_lft;

        // at.R.L = at
        rgt.lft = Some(node.id);

        // at = at.R
        self.update_height(node);
        self.update_height(&mut rgt);

        rgt.id
    }

    // Check balance at a given node and enforce it if necessary with respective rotations.
    fn enforce_balance(&mut self, node: &mut Node<K>) -> u64 {
        let balance = self.get_balance(&node);
        if balance > 1 {
            let mut lft = node.lft.and_then(|id| self.node(id)).unwrap();
            if self.get_balance(&lft) < 0 {
                let rotated = self.rotate_right(&mut lft);
                node.lft = Some(rotated);
            }
            self.rotate_left(node)
        } else if balance < -1 {
            let mut rgt = node.rgt.and_then(|id| self.node(id)).unwrap();
            if self.get_balance(&rgt) > 0 {
                let rotated = self.rotate_left(&mut rgt);
                node.rgt = Some(rotated);
            }
            self.rotate_right(node)
        } else {
            node.id
        }
    }

    // Returns (node, parent node) for a node that holds the `key`.
    // For root node, same node is returned for node and parent node.
    fn lookup_at(&self, mut at: u64, key: &K) -> Option<(Node<K>, Node<K>)> {
        let mut p: Node<K> = self.node(at).unwrap();
        loop {
            match self.node(at) {
                Some(node) => {
                    if node.key.eq(key) {
                        return Some((node, p));
                    } else if node.key.lt(key) {
                        match node.rgt {
                            Some(rgt) => {
                                p = node;
                                at = rgt;
                            }
                            None => break,
                        }
                    } else {
                        match node.lft {
                            Some(lft) => {
                                p = node;
                                at = lft;
                            }
                            None => break,
                        }
                    }
                }
                None => break,
            }
        }
        None
    }

    // Navigate from root to node holding `key` and backtrace back to the root
    // enforcing balance (if necessary) along the way.
    fn check_balance(&mut self, at: u64, key: &K) -> u64 {
        match self.node(at) {
            Some(mut node) => {
                if node.key.eq(key) {
                    self.update_height(&mut node);
                    self.enforce_balance(&mut node)
                } else {
                    if node.key.gt(key) {
                        match node.lft {
                            Some(l) => {
                                let id = self.check_balance(l, key);
                                node.lft = Some(id);
                            }
                            None => (),
                        }
                    } else {
                        match node.rgt {
                            Some(r) => {
                                let id = self.check_balance(r, key);
                                node.rgt = Some(id);
                            }
                            None => (),
                        }
                    }
                    self.update_height(&mut node);
                    self.enforce_balance(&mut node)
                }
            }
            None => at,
        }
    }

    // Node holding the key is not removed from the tree - instead the substitute node is found,
    // the key is copied to 'removed' node from substitute node, and then substitute node gets
    // removed from the tree.
    //
    // The substitute node is either:
    // - right-most (max) node of the left subtree (containing smaller keys) of node holding `key`
    // - or left-most (min) node of the right subtree (containing larger keys) of node holding `key`
    //
    fn do_remove(&mut self, key: &K) -> u64 {
        // r_node - node containing key of interest
        // p_node - immediate parent node of r_node
        let (mut r_node, mut p_node) = match self.lookup_at(self.root, key) {
            Some(x) => x,
            None => return self.root, // cannot remove a missing key, no changes to the tree needed
        };

        let lft_opt = r_node.lft;
        let rgt_opt = r_node.rgt;

        if lft_opt.is_none() && rgt_opt.is_none() {
            // remove leaf
            if p_node.key.lt(key) {
                p_node.rgt = None;
            } else {
                p_node.lft = None;
            }
            self.update_height(&mut p_node);

            self.swap_with_last(r_node.id);

            // removing node might have caused a imbalance - balance the tree up to the root,
            // starting from lowest affected key - the parent of a leaf node in this case
            self.check_balance(self.root, &p_node.key)
        } else {
            // non-leaf node, select subtree to proceed with
            let b = self.get_balance(&r_node);
            if b >= 0 {
                // proceed with left subtree
                let lft = lft_opt.unwrap();

                // k - max key from left subtree
                // n - node that holds key k, p - immediate parent of n
                let (n, mut p) = self.max_at(lft, r_node.id).unwrap();
                let k = n.key.clone();

                if p.rgt.clone().map(|id| id == n.id).unwrap_or_default() {
                    // n is on right link of p
                    p.rgt = n.lft;
                } else {
                    // n is on left link of p
                    p.lft = n.lft;
                }

                self.update_height(&mut p);

                if r_node.id == p.id {
                    // r_node.id and p.id can overlap on small trees (2 levels, 2-3 nodes)
                    // that leads to nasty lost update of the key, refresh below fixes that
                    r_node = self.node(r_node.id).unwrap();
                }
                r_node.key = k;
                self.save(&r_node);

                self.swap_with_last(n.id);

                // removing node might have caused an imbalance - balance the tree up to the root,
                // starting from the lowest affected key (max key from left subtree in this case)
                self.check_balance(self.root, &p.key)
            } else {
                // proceed with right subtree
                let rgt = rgt_opt.unwrap();

                // k - min key from right subtree
                // n - node that holds key k, p - immediate parent of n
                let (n, mut p) = self.min_at(rgt, r_node.id).unwrap();
                let k = n.key.clone();

                if p.lft.map(|id| id == n.id).unwrap_or_default() {
                    // n is on left link of p
                    p.lft = n.rgt;
                } else {
                    // n is on right link of p
                    p.rgt = n.rgt;
                }

                self.update_height(&mut p);

                if r_node.id == p.id {
                    // r_node.id and p.id can overlap on small trees (2 levels, 2-3 nodes)
                    // that leads to nasty lost update of the key, refresh below fixes that
                    r_node = self.node(r_node.id).unwrap();
                }
                r_node.key = k;
                self.save(&r_node);

                self.swap_with_last(n.id);

                // removing node might have caused a imbalance - balance the tree up to the root,
                // starting from the lowest affected key (min key from right subtree in this case)
                self.check_balance(self.root, &p.key)
            }
        }
    }

    // Move content of node with id = `len - 1` (parent left or right link, left, right, key, height)
    // to node with given `id`, and remove node `len - 1` (pop the vector of nodes).
    // This ensures that among `n` nodes in the tree, max `id` is `n-1`, so when new node is inserted,
    // it gets an `id` as its position in the vector.
    fn swap_with_last(&mut self, id: u64) {
        if id == self.len() - 1 {
            // noop: id is already last element in the vector
            self.tree.pop();
            return;
        }

        let key = self.node(self.len() - 1).map(|n| n.key).unwrap();
        let (mut n, mut p) = self.lookup_at(self.root, &key).unwrap();

        if n.id != p.id {
            if p.lft.map(|id| id == n.id).unwrap_or_default() {
                p.lft = Some(id);
            } else {
                p.rgt = Some(id);
            }
            self.save(&p);
        }

        if self.root == n.id {
            self.root = id;
        }

        n.id = id;
        self.save(&n);
        self.tree.pop();
    }
}

impl<'a, K, V> IntoIterator for &'a LegacyTreeMap<K, V>
where
    K: Ord + Clone + BorshSerialize + BorshDeserialize,
    V: BorshSerialize + BorshDeserialize,
{
    type Item = (K, V);
    type IntoIter = Cursor<'a, K, V>;

    fn into_iter(self) -> Self::IntoIter {
        Cursor::asc(self)
    }
}

impl<K, V> Iterator for Cursor<'_, K, V>
where
    K: Ord + Clone + BorshSerialize + BorshDeserialize,
    V: BorshSerialize + BorshDeserialize,
{
    type Item = (K, V);

    fn next(&mut self) -> Option<Self::Item> {
        let this_key = self.key.clone();

        let next_key = self
            .key
            .take()
            .and_then(|k| if self.asc { self.map.higher(&k) } else { self.map.lower(&k) })
            .filter(|k| fits(k, &self.lo, &self.hi));
        self.key = next_key;

        this_key.and_then(|k| self.map.get(&k).map(|v| (k, v)))
    }
}

fn fits<K: Ord>(key: &K, lo: &Bound<K>, hi: &Bound<K>) -> bool {
    (match lo {
        Bound::Included(ref x) => key >= x,
        Bound::Excluded(ref x) => key > x,
        Bound::Unbounded => true,
    }) && (match hi {
        Bound::Included(ref x) => key <= x,
        Bound::Excluded(ref x) => key < x,
        Bound::Unbounded => true,
    })
}

pub struct Cursor<'a, K, V> {
    asc: bool,
    lo: Bound<K>,
    hi: Bound<K>,
    key: Option<K>,
    map: &'a LegacyTreeMap<K, V>,
}

impl<'a, K, V> Cursor<'a, K, V>
where
    K: Ord + Clone + BorshSerialize + BorshDeserialize,
    V: BorshSerialize + BorshDeserialize,
{
    fn asc(map: &'a LegacyTreeMap<K, V>) -> Self {
        let key: Option<K> = map.min();
        Self { asc: true, key, lo: Bound::Unbounded, hi: Bound::Unbounded, map }
    }

    fn asc_from(map: &'a LegacyTreeMap<K, V>, key: K) -> Self {
        let key = map.higher(&key);
        Self { asc: true, key, lo: Bound::Unbounded, hi: Bound::Unbounded, map }
    }

    fn desc(map: &'a LegacyTreeMap<K, V>) -> Self {
        let key: Option<K> = map.max();
        Self { asc: false, key, lo: Bound::Unbounded, hi: Bound::Unbounded, map }
    }

    fn desc_from(map: &'a LegacyTreeMap<K, V>, key: K) -> Self {
        let key = map.lower(&key);
        Self { asc: false, key, lo: Bound::Unbounded, hi: Bound::Unbounded, map }
    }

    fn range(map: &'a LegacyTreeMap<K, V>, lo: Bound<K>, hi: Bound<K>) -> Self {
        let key = match &lo {
            Bound::Included(k) if map.contains_key(k) => Some(k.clone()),
            Bound::Included(k) | Bound::Excluded(k) => map.higher(k),
            _ => None,
        };
        let key = key.filter(|k| fits(k, &lo, &hi));

        Self { asc: true, key, lo, hi, map }
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::{next_trie_id, test_env};

    extern crate rand;
    use self::rand::RngCore;
    use quickcheck::QuickCheck;
    use serde::export::Formatter;
    use std::collections::BTreeMap;
    use std::collections::HashSet;
    use std::fmt::{Debug, Result};

    /// Return height of the tree - number of nodes on the longest path starting from the root node.
    fn height<K, V>(tree: &LegacyTreeMap<K, V>) -> u64
    where
        K: Ord + Clone + BorshSerialize + BorshDeserialize,
        V: BorshSerialize + BorshDeserialize,
    {
        tree.node(tree.root).map(|n| n.ht).unwrap_or_default()
    }

    fn random(n: u64) -> Vec<u32> {
        let mut rng = rand::thread_rng();
        let mut vec = Vec::with_capacity(n as usize);
        (0..n).for_each(|_| {
            vec.push(rng.next_u32() % 1000);
        });
        vec
    }

    fn log2(x: f64) -> f64 {
        std::primitive::f64::log(x, 2.0f64)
    }

    fn max_tree_height(n: u64) -> u64 {
        // h <= C * log2(n + D) + B
        // where:
        // C =~ 1.440, D =~ 1.065, B =~ 0.328
        // (source: https://en.wikipedia.org/wiki/AVL_tree)
        const B: f64 = -0.328;
        const C: f64 = 1.440;
        const D: f64 = 1.065;

        let h = C * log2(n as f64 + D) + B;
        h.ceil() as u64
    }

    impl<K> Debug for Node<K>
    where
        K: Ord + Clone + Debug + BorshSerialize + BorshDeserialize,
    {
        fn fmt(&self, f: &mut Formatter<'_>) -> Result {
            f.debug_struct("Node")
                .field("id", &self.id)
                .field("key", &self.key)
                .field("lft", &self.lft)
                .field("rgt", &self.rgt)
                .field("ht", &self.ht)
                .finish()
        }
    }

    impl<K, V> Debug for LegacyTreeMap<K, V>
    where
        K: Ord + Clone + Debug + BorshSerialize + BorshDeserialize,
        V: Debug + BorshSerialize + BorshDeserialize,
    {
        fn fmt(&self, f: &mut Formatter<'_>) -> Result {
            f.debug_struct("TreeMap")
                .field("root", &self.root)
                .field("tree", &self.tree.iter().collect::<Vec<Node<K>>>())
                .field("val", &self.val.iter().collect::<Vec<(K, V)>>())
                .finish()
        }
    }

    #[test]
    fn test_empty() {
        test_env::setup();

        let map: LegacyTreeMap<u8, u8> = LegacyTreeMap::new(vec![b't']);
        assert_eq!(map.len(), 0);
        assert_eq!(height(&map), 0);
        assert_eq!(map.get(&42), None);
        assert!(!map.contains_key(&42));
        assert_eq!(map.min(), None);
        assert_eq!(map.max(), None);
        assert_eq!(map.lower(&42), None);
        assert_eq!(map.higher(&42), None);
    }

    #[test]
    fn test_insert_3_rotate_l_l() {
        test_env::setup();

        let mut map: LegacyTreeMap<i32, i32> = LegacyTreeMap::new(next_trie_id());
        assert_eq!(height(&map), 0);

        map.insert(&3, &3);
        assert_eq!(height(&map), 1);

        map.insert(&2, &2);
        assert_eq!(height(&map), 2);

        map.insert(&1, &1);
        assert_eq!(height(&map), 2);

        let root = map.root;
        assert_eq!(root, 1);
        assert_eq!(map.node(root).map(|n| n.key), Some(2));

        map.clear();
    }

    #[test]
    fn test_insert_3_rotate_r_r() {
        test_env::setup();

        let mut map: LegacyTreeMap<i32, i32> = LegacyTreeMap::new(next_trie_id());
        assert_eq!(height(&map), 0);

        map.insert(&1, &1);
        assert_eq!(height(&map), 1);

        map.insert(&2, &2);
        assert_eq!(height(&map), 2);

        map.insert(&3, &3);

        let root = map.root;
        assert_eq!(root, 1);
        assert_eq!(map.node(root).map(|n| n.key), Some(2));
        assert_eq!(height(&map), 2);

        map.clear();
    }

    #[test]
    fn test_insert_lookup_n_asc() {
        test_env::setup();

        let mut map: LegacyTreeMap<i32, i32> = LegacyTreeMap::new(next_trie_id());

        let n: u64 = 30;
        let cases = (0..2 * (n as i32)).collect::<Vec<i32>>();

        let mut counter = 0;
        for k in &cases {
            if *k % 2 == 0 {
                counter += 1;
                map.insert(k, &counter);
            }
        }

        counter = 0;
        for k in &cases {
            if *k % 2 == 0 {
                counter += 1;
                assert_eq!(map.get(k), Some(counter));
            } else {
                assert_eq!(map.get(k), None);
            }
        }

        assert!(height(&map) <= max_tree_height(n));
        map.clear();
    }

    #[test]
    fn test_insert_lookup_n_desc() {
        test_env::setup();

        let mut map: LegacyTreeMap<i32, i32> = LegacyTreeMap::new(next_trie_id());

        let n: u64 = 30;
        let cases = (0..2 * (n as i32)).rev().collect::<Vec<i32>>();

        let mut counter = 0;
        for k in &cases {
            if *k % 2 == 0 {
                counter += 1;
                map.insert(k, &counter);
            }
        }

        counter = 0;
        for k in &cases {
            if *k % 2 == 0 {
                counter += 1;
                assert_eq!(map.get(k), Some(counter));
            } else {
                assert_eq!(map.get(k), None);
            }
        }

        assert!(height(&map) <= max_tree_height(n));
        map.clear();
    }

    #[test]
    fn insert_n_random() {
        test_env::setup_free();

        for k in 1..10 {
            // tree size is 2^k
            let mut map: LegacyTreeMap<u32, u32> = LegacyTreeMap::new(next_trie_id());

            let n = 1 << k;
            let input: Vec<u32> = random(n);

            for x in &input {
                map.insert(x, &42);
            }

            for x in &input {
                assert_eq!(map.get(x), Some(42));
            }

            assert!(height(&map) <= max_tree_height(n));
            map.clear();
        }
    }

    #[test]
    fn test_min() {
        test_env::setup();

        let n: u64 = 30;
        let vec = random(n);

        let mut map: LegacyTreeMap<u32, u32> = LegacyTreeMap::new(vec![b't']);
        for x in vec.iter().rev() {
            map.insert(x, &1);
        }

        assert_eq!(map.min().unwrap(), *vec.iter().min().unwrap());
        map.clear();
    }

    #[test]
    fn test_max() {
        test_env::setup();

        let n: u64 = 30;
        let vec = random(n);

        let mut map: LegacyTreeMap<u32, u32> = LegacyTreeMap::new(vec![b't']);
        for x in vec.iter().rev() {
            map.insert(x, &1);
        }

        assert_eq!(map.max().unwrap(), *vec.iter().max().unwrap());
        map.clear();
    }

    #[test]
    fn test_lower() {
        test_env::setup();

        let mut map: LegacyTreeMap<u32, u32> = LegacyTreeMap::new(next_trie_id());
        let vec: Vec<u32> = vec![10, 20, 30, 40, 50];

        for x in vec.iter() {
            map.insert(x, &1);
        }

        assert_eq!(map.lower(&5), None);
        assert_eq!(map.lower(&10), None);
        assert_eq!(map.lower(&11), Some(10));
        assert_eq!(map.lower(&20), Some(10));
        assert_eq!(map.lower(&49), Some(40));
        assert_eq!(map.lower(&50), Some(40));
        assert_eq!(map.lower(&51), Some(50));

        map.clear();
    }

    #[test]
    fn test_higher() {
        test_env::setup();

        let mut map: LegacyTreeMap<u32, u32> = LegacyTreeMap::new(next_trie_id());
        let vec: Vec<u32> = vec![10, 20, 30, 40, 50];

        for x in vec.iter() {
            map.insert(x, &1);
        }

        assert_eq!(map.higher(&5), Some(10));
        assert_eq!(map.higher(&10), Some(20));
        assert_eq!(map.higher(&11), Some(20));
        assert_eq!(map.higher(&20), Some(30));
        assert_eq!(map.higher(&49), Some(50));
        assert_eq!(map.higher(&50), None);
        assert_eq!(map.higher(&51), None);

        map.clear();
    }

    #[test]
    fn test_floor_key() {
        test_env::setup();

        let mut map: LegacyTreeMap<u32, u32> = LegacyTreeMap::new(next_trie_id());
        let vec: Vec<u32> = vec![10, 20, 30, 40, 50];

        for x in vec.iter() {
            map.insert(x, &1);
        }

        assert_eq!(map.floor_key(&5), None);
        assert_eq!(map.floor_key(&10), Some(10));
        assert_eq!(map.floor_key(&11), Some(10));
        assert_eq!(map.floor_key(&20), Some(20));
        assert_eq!(map.floor_key(&49), Some(40));
        assert_eq!(map.floor_key(&50), Some(50));
        assert_eq!(map.floor_key(&51), Some(50));

        map.clear();
    }

    #[test]
    fn test_ceil_key() {
        test_env::setup();

        let mut map: LegacyTreeMap<u32, u32> = LegacyTreeMap::new(next_trie_id());
        let vec: Vec<u32> = vec![10, 20, 30, 40, 50];

        for x in vec.iter() {
            map.insert(x, &1);
        }

        assert_eq!(map.ceil_key(&5), Some(10));
        assert_eq!(map.ceil_key(&10), Some(10));
        assert_eq!(map.ceil_key(&11), Some(20));
        assert_eq!(map.ceil_key(&20), Some(20));
        assert_eq!(map.ceil_key(&49), Some(50));
        assert_eq!(map.ceil_key(&50), Some(50));
        assert_eq!(map.ceil_key(&51), None);

        map.clear();
    }

    #[test]
    fn test_remove_1() {
        test_env::setup();

        let mut map: LegacyTreeMap<u32, u32> = LegacyTreeMap::new(next_trie_id());
        map.insert(&1, &1);
        assert_eq!(map.get(&1), Some(1));
        map.remove(&1);
        assert_eq!(map.get(&1), None);
        assert_eq!(map.tree.len(), 0);
        map.clear();
    }

    #[test]
    fn test_remove_3() {
        test_env::setup();

        let map: LegacyTreeMap<u32, u32> = avl(&[(0, 0)], &[0, 0, 1]);

        assert_eq!(map.iter().collect::<Vec<(u32, u32)>>(), vec![]);
    }

    #[test]
    fn test_remove_3_desc() {
        test_env::setup();

        let vec: Vec<u32> = vec![3, 2, 1];
        let mut map: LegacyTreeMap<u32, u32> = LegacyTreeMap::new(next_trie_id());

        for x in &vec {
            assert_eq!(map.get(x), None);
            map.insert(x, &1);
            assert_eq!(map.get(x), Some(1));
        }

        for x in &vec {
            assert_eq!(map.get(x), Some(1));
            map.remove(x);
            assert_eq!(map.get(x), None);
        }
        map.clear();
    }

    #[test]
    fn test_remove_3_asc() {
        test_env::setup();

        let vec: Vec<u32> = vec![1, 2, 3];
        let mut map: LegacyTreeMap<u32, u32> = LegacyTreeMap::new(next_trie_id());

        for x in &vec {
            assert_eq!(map.get(x), None);
            map.insert(x, &1);
            assert_eq!(map.get(x), Some(1));
        }

        for x in &vec {
            assert_eq!(map.get(x), Some(1));
            map.remove(x);
            assert_eq!(map.get(x), None);
        }
        map.clear();
    }

    #[test]
    fn test_remove_7_regression_1() {
        test_env::setup();

        let vec: Vec<u32> =
            vec![2104297040, 552624607, 4269683389, 3382615941, 155419892, 4102023417, 1795725075];
        let mut map: LegacyTreeMap<u32, u32> = LegacyTreeMap::new(next_trie_id());

        for x in &vec {
            assert_eq!(map.get(x), None);
            map.insert(x, &1);
            assert_eq!(map.get(x), Some(1));
        }

        for x in &vec {
            assert_eq!(map.get(x), Some(1));
            map.remove(x);
            assert_eq!(map.get(x), None);
        }
        map.clear();
    }

    #[test]
    fn test_remove_7_regression_2() {
        test_env::setup();

        let vec: Vec<u32> =
            vec![700623085, 87488544, 1500140781, 1111706290, 3187278102, 4042663151, 3731533080];
        let mut map: LegacyTreeMap<u32, u32> = LegacyTreeMap::new(next_trie_id());

        for x in &vec {
            assert_eq!(map.get(x), None);
            map.insert(x, &1);
            assert_eq!(map.get(x), Some(1));
        }

        for x in &vec {
            assert_eq!(map.get(x), Some(1));
            map.remove(x);
            assert_eq!(map.get(x), None);
        }
        map.clear();
    }

    #[test]
    fn test_remove_9_regression() {
        test_env::setup();

        let vec: Vec<u32> = vec![
            1186903464, 506371929, 1738679820, 1883936615, 1815331350, 1512669683, 3581743264,
            1396738166, 1902061760,
        ];
        let mut map: LegacyTreeMap<u32, u32> = LegacyTreeMap::new(next_trie_id());

        for x in &vec {
            assert_eq!(map.get(x), None);
            map.insert(x, &1);
            assert_eq!(map.get(x), Some(1));
        }

        for x in &vec {
            assert_eq!(map.get(x), Some(1));
            map.remove(x);
            assert_eq!(map.get(x), None);
        }
        map.clear();
    }

    #[test]
    fn test_remove_20_regression_1() {
        test_env::setup();

        let vec: Vec<u32> = vec![
            552517392, 3638992158, 1015727752, 2500937532, 638716734, 586360620, 2476692174,
            1425948996, 3608478547, 757735878, 2709959928, 2092169539, 3620770200, 783020918,
            1986928932, 200210441, 1972255302, 533239929, 497054557, 2137924638,
        ];
        let mut map: LegacyTreeMap<u32, u32> = LegacyTreeMap::new(next_trie_id());

        for x in &vec {
            assert_eq!(map.get(x), None);
            map.insert(x, &1);
            assert_eq!(map.get(x), Some(1));
        }

        for x in &vec {
            assert_eq!(map.get(x), Some(1));
            map.remove(x);
            assert_eq!(map.get(x), None);
        }
        map.clear();
    }

    #[test]
    fn test_remove_7_regression() {
        test_env::setup();

        let vec: Vec<u32> = vec![280, 606, 163, 857, 436, 508, 44, 801];

        let mut map: LegacyTreeMap<u32, u32> = LegacyTreeMap::new(next_trie_id());

        for x in &vec {
            assert_eq!(map.get(x), None);
            map.insert(x, &1);
            assert_eq!(map.get(x), Some(1));
        }

        for x in &vec {
            assert_eq!(map.get(x), Some(1));
            map.remove(x);
            assert_eq!(map.get(x), None);
        }

        assert_eq!(map.len(), 0, "map.len() > 0");
        assert_eq!(map.val.len(), 0, "map.val is not empty");
        assert_eq!(map.tree.len(), 0, "map.tree is not empty");
        map.clear();
    }

    #[test]
    fn test_insert_8_remove_4_regression() {
        let insert = vec![882, 398, 161, 76];
        let remove = vec![242, 687, 860, 811];

        test_env::setup();
        let mut map: LegacyTreeMap<u32, u32> = LegacyTreeMap::new(next_trie_id());

        for (i, (k1, k2)) in insert.iter().zip(remove.iter()).enumerate() {
            let v = i as u32;
            map.insert(k1, &v);
            map.insert(k2, &v);
        }

        for k in remove.iter() {
            map.remove(k);
        }

        assert_eq!(map.len(), insert.len() as u64);

        for (i, k) in insert.iter().enumerate() {
            assert_eq!(map.get(k), Some(i as u32));
        }
    }

    #[test]
    fn test_remove_n() {
        test_env::setup();

        let n: u64 = 20;
        let vec = random(n);

        let mut set: HashSet<u32> = HashSet::new();
        let mut map: LegacyTreeMap<u32, u32> = LegacyTreeMap::new(next_trie_id());
        for x in &vec {
            map.insert(x, &1);
            set.insert(*x);
        }

        assert_eq!(map.len(), set.len() as u64);

        for x in &set {
            assert_eq!(map.get(x), Some(1));
            map.remove(x);
            assert_eq!(map.get(x), None);
        }

        assert_eq!(map.len(), 0, "map.len() > 0");
        assert_eq!(map.tree.len(), 0, "map.tree is not empty");
        assert_eq!(map.val.len(), 0, "map.val is not empty");
        map.clear();
    }

    #[test]
    fn test_remove_root_3() {
        test_env::setup();

        let mut map: LegacyTreeMap<u32, u32> = LegacyTreeMap::new(next_trie_id());
        map.insert(&2, &1);
        map.insert(&3, &1);
        map.insert(&1, &1);
        map.insert(&4, &1);

        map.remove(&2);

        assert_eq!(map.get(&1), Some(1));
        assert_eq!(map.get(&2), None);
        assert_eq!(map.get(&3), Some(1));
        assert_eq!(map.get(&4), Some(1));
        map.clear();
    }

    #[test]
    fn test_insert_2_remove_2_regression() {
        test_env::setup();

        let ins: Vec<u32> = vec![11760225, 611327897];
        let rem: Vec<u32> = vec![2982517385, 1833990072];

        let mut map: LegacyTreeMap<u32, u32> = LegacyTreeMap::new(next_trie_id());
        map.insert(&ins[0], &1);
        map.insert(&ins[1], &1);

        map.remove(&rem[0]);
        map.remove(&rem[1]);

        let h = height(&map);
        let h_max = max_tree_height(map.len());
        assert!(h <= h_max, "h={} h_max={}", h, h_max);
        map.clear();
    }

    #[test]
    fn test_insert_n_duplicates() {
        test_env::setup();
        let mut map: LegacyTreeMap<u32, u32> = LegacyTreeMap::new(next_trie_id());

        for x in 0..30 {
            map.insert(&x, &x);
            map.insert(&42, &x);
        }

        assert_eq!(map.get(&42), Some(29));
        assert_eq!(map.len(), 31);
        assert_eq!(map.val.len(), 31);
        assert_eq!(map.tree.len(), 31);

        map.clear();
    }

    #[test]
    fn test_insert_2n_remove_n_random() {
        test_env::setup();

        for k in 1..4 {
            let mut map: LegacyTreeMap<u32, u32> = LegacyTreeMap::new(next_trie_id());
            let mut set: HashSet<u32> = HashSet::new();

            let n = 1 << k;
            let ins: Vec<u32> = random(n);
            let rem: Vec<u32> = random(n);

            for x in &ins {
                set.insert(*x);
                map.insert(x, &42);
            }

            for x in &rem {
                set.insert(*x);
                map.insert(x, &42);
            }

            for x in &rem {
                set.remove(x);
                map.remove(x);
            }

            assert_eq!(map.len(), set.len() as u64);

            let h = height(&map);
            let h_max = max_tree_height(n);
            assert!(h <= h_max, "[n={}] tree is too high: {} (max is {}).", n, h, h_max);

            map.clear();
        }
    }

    #[test]
    fn test_remove_empty() {
        test_env::setup();
        let mut map: LegacyTreeMap<u32, u32> = LegacyTreeMap::new(next_trie_id());
        assert_eq!(map.remove(&1), None);
    }

    #[test]
    fn test_to_vec() {
        test_env::setup();
        let mut map: LegacyTreeMap<u32, u32> = LegacyTreeMap::new(next_trie_id());
        map.insert(&1, &41);
        map.insert(&2, &42);
        map.insert(&3, &43);

        assert_eq!(map.to_vec(), vec![(1, 41), (2, 42), (3, 43)]);
        map.clear();
    }

    #[test]
    fn test_to_vec_empty() {
        test_env::setup();
        let map: LegacyTreeMap<u32, u32> = LegacyTreeMap::new(next_trie_id());
        assert!(map.to_vec().is_empty());
    }

    #[test]
    fn test_iter() {
        test_env::setup();
        let mut map: LegacyTreeMap<u32, u32> = LegacyTreeMap::new(next_trie_id());
        map.insert(&1, &41);
        map.insert(&2, &42);
        map.insert(&3, &43);

        assert_eq!(map.iter().collect::<Vec<(u32, u32)>>(), vec![(1, 41), (2, 42), (3, 43)]);
        map.clear();
    }

    #[test]
    fn test_iter_empty() {
        test_env::setup();
        let map: LegacyTreeMap<u32, u32> = LegacyTreeMap::new(next_trie_id());
        assert!(map.iter().collect::<Vec<(u32, u32)>>().is_empty());
    }

    #[test]
    fn test_iter_rev() {
        test_env::setup();
        let mut map: LegacyTreeMap<u32, u32> = LegacyTreeMap::new(next_trie_id());
        map.insert(&1, &41);
        map.insert(&2, &42);
        map.insert(&3, &43);

        assert_eq!(map.iter_rev().collect::<Vec<(u32, u32)>>(), vec![(3, 43), (2, 42), (1, 41)]);
        map.clear();
    }

    #[test]
    fn test_iter_rev_empty() {
        test_env::setup();
        let map: LegacyTreeMap<u32, u32> = LegacyTreeMap::new(next_trie_id());
        assert!(map.iter_rev().collect::<Vec<(u32, u32)>>().is_empty());
    }

    #[test]
    fn test_iter_from() {
        test_env::setup();
        let mut map: LegacyTreeMap<u32, u32> = LegacyTreeMap::new(next_trie_id());

        let one: Vec<u32> = vec![10, 20, 30, 40, 50];
        let two: Vec<u32> = vec![45, 35, 25, 15, 5];

        for x in &one {
            map.insert(x, &42);
        }

        for x in &two {
            map.insert(x, &42);
        }

        assert_eq!(
            map.iter_from(29).collect::<Vec<(u32, u32)>>(),
            vec![(30, 42), (35, 42), (40, 42), (45, 42), (50, 42)]
        );

        assert_eq!(
            map.iter_from(30).collect::<Vec<(u32, u32)>>(),
            vec![(35, 42), (40, 42), (45, 42), (50, 42)]
        );

        assert_eq!(
            map.iter_from(31).collect::<Vec<(u32, u32)>>(),
            vec![(35, 42), (40, 42), (45, 42), (50, 42)]
        );
        map.clear();
    }

    #[test]
    fn test_iter_from_empty() {
        test_env::setup();
        let map: LegacyTreeMap<u32, u32> = LegacyTreeMap::new(next_trie_id());
        assert!(map.iter_from(42).collect::<Vec<(u32, u32)>>().is_empty());
    }

    #[test]
    fn test_iter_rev_from() {
        test_env::setup();
        let mut map: LegacyTreeMap<u32, u32> = LegacyTreeMap::new(next_trie_id());

        let one: Vec<u32> = vec![10, 20, 30, 40, 50];
        let two: Vec<u32> = vec![45, 35, 25, 15, 5];

        for x in &one {
            map.insert(x, &42);
        }

        for x in &two {
            map.insert(x, &42);
        }

        assert_eq!(
            map.iter_rev_from(29).collect::<Vec<(u32, u32)>>(),
            vec![(25, 42), (20, 42), (15, 42), (10, 42), (5, 42)]
        );

        assert_eq!(
            map.iter_rev_from(30).collect::<Vec<(u32, u32)>>(),
            vec![(25, 42), (20, 42), (15, 42), (10, 42), (5, 42)]
        );

        assert_eq!(
            map.iter_rev_from(31).collect::<Vec<(u32, u32)>>(),
            vec![(30, 42), (25, 42), (20, 42), (15, 42), (10, 42), (5, 42)]
        );
        map.clear();
    }

    #[test]
    fn test_range() {
        test_env::setup();
        let mut map: LegacyTreeMap<u32, u32> = LegacyTreeMap::new(next_trie_id());

        let one: Vec<u32> = vec![10, 20, 30, 40, 50];
        let two: Vec<u32> = vec![45, 35, 25, 15, 5];

        for x in &one {
            map.insert(x, &42);
        }

        for x in &two {
            map.insert(x, &42);
        }

        assert_eq!(
            map.range((Bound::Included(20), Bound::Excluded(30))).collect::<Vec<(u32, u32)>>(),
            vec![(20, 42), (25, 42)]
        );

        assert_eq!(
            map.range((Bound::Excluded(10), Bound::Included(40))).collect::<Vec<(u32, u32)>>(),
            vec![(15, 42), (20, 42), (25, 42), (30, 42), (35, 42), (40, 42)]
        );

        assert_eq!(
            map.range((Bound::Included(20), Bound::Included(40))).collect::<Vec<(u32, u32)>>(),
            vec![(20, 42), (25, 42), (30, 42), (35, 42), (40, 42)]
        );

        assert_eq!(
            map.range((Bound::Excluded(20), Bound::Excluded(45))).collect::<Vec<(u32, u32)>>(),
            vec![(25, 42), (30, 42), (35, 42), (40, 42)]
        );

        assert_eq!(
            map.range((Bound::Excluded(20), Bound::Excluded(45))).collect::<Vec<(u32, u32)>>(),
            vec![(25, 42), (30, 42), (35, 42), (40, 42)]
        );

        assert_eq!(
            map.range((Bound::Excluded(25), Bound::Excluded(30))).collect::<Vec<(u32, u32)>>(),
            vec![]
        );

        assert_eq!(
            map.range((Bound::Included(25), Bound::Included(25))).collect::<Vec<(u32, u32)>>(),
            vec![(25, 42)]
        );

        assert_eq!(
            map.range((Bound::Excluded(25), Bound::Included(25))).collect::<Vec<(u32, u32)>>(),
            vec![]
        ); // the range makes no sense, but `BTreeMap` does not panic in this case

        map.clear();
    }

    #[test]
    #[should_panic(expected = "Invalid range.")]
    fn test_range_panics_same_excluded() {
        test_env::setup();
        let map: LegacyTreeMap<u32, u32> = LegacyTreeMap::new(next_trie_id());
        let _ = map.range((Bound::Excluded(1), Bound::Excluded(1)));
    }

    #[test]
    #[should_panic(expected = "Invalid range.")]
    fn test_range_panics_non_overlap_incl_exlc() {
        test_env::setup();
        let map: LegacyTreeMap<u32, u32> = LegacyTreeMap::new(next_trie_id());
        let _ = map.range((Bound::Included(2), Bound::Excluded(1)));
    }

    #[test]
    #[should_panic(expected = "Invalid range.")]
    fn test_range_panics_non_overlap_excl_incl() {
        test_env::setup();
        let map: LegacyTreeMap<u32, u32> = LegacyTreeMap::new(next_trie_id());
        let _ = map.range((Bound::Excluded(2), Bound::Included(1)));
    }

    #[test]
    #[should_panic(expected = "Invalid range.")]
    fn test_range_panics_non_overlap_incl_incl() {
        test_env::setup();
        let map: LegacyTreeMap<u32, u32> = LegacyTreeMap::new(next_trie_id());
        let _ = map.range((Bound::Included(2), Bound::Included(1)));
    }

    #[test]
    fn test_iter_rev_from_empty() {
        test_env::setup();
        let map: LegacyTreeMap<u32, u32> = LegacyTreeMap::new(next_trie_id());
        assert!(map.iter_rev_from(42).collect::<Vec<(u32, u32)>>().is_empty());
    }

    #[test]
    fn test_balance_regression_1() {
        let insert = vec![(2, 0), (3, 0), (4, 0)];
        let remove = vec![0, 0, 0, 1];

        let map = avl(&insert, &remove);
        assert!(is_balanced(&map, map.root));
    }

    #[test]
    fn test_balance_regression_2() {
        let insert = vec![(1, 0), (2, 0), (0, 0), (3, 0), (5, 0), (6, 0)];
        let remove = vec![0, 0, 0, 3, 5, 6, 7, 4];

        let map = avl(&insert, &remove);
        assert!(is_balanced(&map, map.root));
    }

    //
    // Property-based tests of AVL-based TreeMap against std::collections::BTreeMap
    //

    fn avl<K, V>(insert: &[(K, V)], remove: &[K]) -> LegacyTreeMap<K, V>
    where
        K: Ord + Clone + BorshSerialize + BorshDeserialize,
        V: Default + BorshSerialize + BorshDeserialize,
    {
        test_env::setup_free();
        let mut map: LegacyTreeMap<K, V> = LegacyTreeMap::new(next_trie_id());
        for k in remove {
            map.insert(k, &Default::default());
        }
        let n = insert.len().max(remove.len());
        for i in 0..n {
            if i < remove.len() {
                map.remove(&remove[i]);
            }
            if i < insert.len() {
                let (k, v) = &insert[i];
                map.insert(k, v);
            }
        }
        map
    }

    fn rb<K, V>(insert: &[(K, V)], remove: &[K]) -> BTreeMap<K, V>
    where
        K: Ord + Clone + BorshSerialize + BorshDeserialize,
        V: Clone + Default + BorshSerialize + BorshDeserialize,
    {
        let mut map: BTreeMap<K, V> = BTreeMap::default();
        for k in remove {
            map.insert(k.clone(), Default::default());
        }
        let n = insert.len().max(remove.len());
        for i in 0..n {
            if i < remove.len() {
                map.remove(&remove[i]);
            }
            if i < insert.len() {
                let (k, v) = &insert[i];
                map.insert(k.clone(), v.clone());
            }
        }
        map
    }

    #[test]
    fn prop_avl_vs_rb() {
        fn prop(insert: Vec<(u32, u32)>, remove: Vec<u32>) -> bool {
            let a = avl(&insert, &remove);
            let b = rb(&insert, &remove);
            let v1: Vec<(u32, u32)> = a.iter().collect();
            let v2: Vec<(u32, u32)> = b.into_iter().collect();
            v1 == v2
        }

        QuickCheck::new()
            .tests(300)
            .quickcheck(prop as fn(std::vec::Vec<(u32, u32)>, std::vec::Vec<u32>) -> bool);
    }

    fn is_balanced<K, V>(map: &LegacyTreeMap<K, V>, root: u64) -> bool
    where
        K: Debug + Ord + Clone + BorshSerialize + BorshDeserialize,
        V: Debug + BorshSerialize + BorshDeserialize,
    {
        let node = map.node(root).unwrap();
        let balance = map.get_balance(&node);

        (balance >= -1 && balance <= 1)
            && node.lft.map(|id| is_balanced(map, id)).unwrap_or(true)
            && node.rgt.map(|id| is_balanced(map, id)).unwrap_or(true)
    }

    #[test]
    fn prop_avl_balance() {
        test_env::setup_free();

        fn prop(insert: Vec<(u32, u32)>, remove: Vec<u32>) -> bool {
            let map = avl(&insert, &remove);
            map.len() == 0 || is_balanced(&map, map.root)
        }

        QuickCheck::new()
            .tests(300)
            .quickcheck(prop as fn(std::vec::Vec<(u32, u32)>, std::vec::Vec<u32>) -> bool);
    }

    #[test]
    fn prop_avl_height() {
        test_env::setup_free();

        fn prop(insert: Vec<(u32, u32)>, remove: Vec<u32>) -> bool {
            let map = avl(&insert, &remove);
            height(&map) <= max_tree_height(map.len())
        }

        QuickCheck::new()
            .tests(300)
            .quickcheck(prop as fn(std::vec::Vec<(u32, u32)>, std::vec::Vec<u32>) -> bool);
    }

    fn range_prop(
        insert: Vec<(u32, u32)>,
        remove: Vec<u32>,
        range: (Bound<u32>, Bound<u32>),
    ) -> bool {
        let a = avl(&insert, &remove);
        let b = rb(&insert, &remove);
        let v1: Vec<(u32, u32)> = a.range(range).collect();
        let v2: Vec<(u32, u32)> = b.range(range).map(|(k, v)| (*k, *v)).collect();
        v1 == v2
    }

    type Prop = fn(std::vec::Vec<(u32, u32)>, std::vec::Vec<u32>, u32, u32) -> bool;

    #[test]
    fn prop_avl_vs_rb_range_incl_incl() {
        fn prop(insert: Vec<(u32, u32)>, remove: Vec<u32>, r1: u32, r2: u32) -> bool {
            let range = (Bound::Included(r1.min(r2)), Bound::Included(r1.max(r2)));
            range_prop(insert, remove, range)
        }

        QuickCheck::new().tests(300).quickcheck(prop as Prop);
    }

    #[test]
    fn prop_avl_vs_rb_range_incl_excl() {
        fn prop(insert: Vec<(u32, u32)>, remove: Vec<u32>, r1: u32, r2: u32) -> bool {
            let range = (Bound::Included(r1.min(r2)), Bound::Excluded(r1.max(r2)));
            range_prop(insert, remove, range)
        }

        QuickCheck::new().tests(300).quickcheck(prop as Prop);
    }

    #[test]
    fn prop_avl_vs_rb_range_excl_incl() {
        fn prop(insert: Vec<(u32, u32)>, remove: Vec<u32>, r1: u32, r2: u32) -> bool {
            let range = (Bound::Excluded(r1.min(r2)), Bound::Included(r1.max(r2)));
            range_prop(insert, remove, range)
        }

        QuickCheck::new().tests(300).quickcheck(prop as Prop);
    }

    #[test]
    fn prop_avl_vs_rb_range_excl_excl() {
        fn prop(insert: Vec<(u32, u32)>, remove: Vec<u32>, r1: u32, r2: u32) -> bool {
            // (Excluded(x), Excluded(x)) is invalid range, checking against it makes no sense
            r1 == r2 || {
                let range = (Bound::Excluded(r1.min(r2)), Bound::Excluded(r1.max(r2)));
                range_prop(insert, remove, range)
            }
        }

        QuickCheck::new().tests(300).quickcheck(prop as Prop);
    }
}
