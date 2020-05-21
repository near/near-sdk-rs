
use borsh::{BorshDeserialize, BorshSerialize};

use crate::collections::{append, next_trie_id, serialize};
use crate::collections::UnorderedMap;
use crate::env;

/// AVL tree implementation
///
/// Runtime complexity (N = number of entries):
/// - `lookup`/`insert`/`remove`: O(log(N)) worst case
/// - `min`/`max`: O(log(N)) worst case
/// - `floor`/`ceil` (find closes key above/below): O(log(N)) worst case
/// - iterate keys in sorted order: O(Nlog(N)) worst case
///
#[derive(BorshSerialize, BorshDeserialize)]
pub struct TreeMap<K, V> {
    tree_prefix: Vec<u8>,

    len: u64,
    root: u64,                      // ID of a root node of the tree
    ht: UnorderedMap<u64, u64>,     // height of a subtree at a node
    lft: UnorderedMap<u64, u64>,    // left link of a node
    rgt: UnorderedMap<u64, u64>,    // right link of a node
    key: UnorderedMap<u64, K>,      // key value stored in a node
    val: UnorderedMap<K, V>,        // value associated with key
}

impl<K, V> Default for TreeMap<K, V>
    where
        K: Ord + BorshSerialize + BorshDeserialize,
        V: BorshSerialize + BorshDeserialize,
{
    fn default() -> Self {
        Self::new(next_trie_id())
    }
}


impl<K, V> TreeMap<K, V>
    where
        K: Ord + BorshSerialize + BorshDeserialize,
        V: BorshSerialize + BorshDeserialize,
{
    pub fn new(id: Vec<u8>) -> Self {
        let h_prefix = append(&id, b'h');
        let l_prefix = append(&id, b'l');
        let r_prefix = append(&id, b'r');
        let k_prefix = append(&id, b'k');
        let v_prefix = append(&id, b'v');

        Self {
            tree_prefix: id,
            root: 0,
            len: 0,
            ht: UnorderedMap::new(h_prefix),
            lft: UnorderedMap::new(l_prefix),
            rgt: UnorderedMap::new(r_prefix),
            key: UnorderedMap::new(k_prefix),
            val: UnorderedMap::new(v_prefix),
        }
    }

    pub fn len(&self) -> u64 {
        self.len
    }

    pub fn clear(&mut self) {
        self.ht.clear();
        self.lft.clear();
        self.rgt.clear();
        self.key.clear();
        self.val.clear();
        self.len = 0;
        self.root = 0;
    }

    pub fn height(&mut self) -> u64 {
        self.ht.get(&self.root).unwrap_or_default()
    }

    pub fn get(&self, key: &K) -> Option<V> {
        self.val.get(key)
    }

    pub fn insert(&mut self, key: K, val: V) -> Option<V> {
        if self.val.get(&key).is_some() {
            // key is already present, changing only associated value
            self.val.insert(&key, &val)
        } else {
            let at = self.root;
            let id = self.len;
            let root = self.insert_at(at, id, &key);
            self.set_root(root);
            self.len += 1;
            self.val.insert(&key, &val)
        }
    }

    pub fn remove(&mut self, _key: K) -> Option<V> {
        None // TODO
    }

    pub fn min(&self) -> Option<K> {
        self.min_at(self.root)
    }

    pub fn max(&self) -> Option<K> {
        self.max_at(self.root)
    }

    pub fn floor(&self, key: &K) -> Option<K> {
        self.floor_at(self.root, key)
    }

    pub fn ceil(&self, key: &K) -> Option<K> {
        self.ceil_at(self.root, key)
    }

    pub fn iter(&self, _key: &K) -> impl Iterator<Item=K> {
        // self.min() and continue with self.floor()
        std::iter::empty() // TODO
    }

    pub fn iter_rev(&self, _key: &K) -> impl Iterator<Item=K> {
        // self.max() and continue with self.ceil()
        std::iter::empty() // TODO
    }

    //
    // Internal utilities
    //

    fn set_root(&mut self, root: u64) {
        env::storage_write(&self.tree_prefix, &serialize(&root));
        self.root = root;
    }

    pub fn min_at(&self, mut at: u64) -> Option<K> {
        loop {
            match self.lft.get(&at) {
                Some(lft) => at = lft,
                None => break
            }
        }
        self.key.get(&at)
    }

    pub fn max_at(&self, mut at: u64) -> Option<K> {
        loop {
            match self.rgt.get(&at) {
                Some(rgt) => at = rgt,
                None => break
            }
        }
        self.key.get(&at)
    }

    pub fn floor_at(&self, mut at: u64, key: &K) -> Option<K> {
        let mut seen: Option<K> = None;
        loop {
            match self.key.get(&at) {
                Some(k) => {
                    if k.le(key) {
                        match self.rgt.get(&at) {
                            Some(rgt) => at = rgt,
                            None => break
                        }
                    } else {
                        seen = Some(k);
                        match self.lft.get(&at) {
                            Some(lft) => at = lft,
                            None => break
                        }
                    }
                },
                None => break
            }
        }
        seen
    }

    pub fn ceil_at(&self, mut at: u64, key: &K) -> Option<K> {
        let mut seen: Option<K> = None;
        loop {
            match self.key.get(&at) {
                Some(k) => {
                    if k.lt(key) {
                        seen = Some(k);
                        match self.rgt.get(&at) {
                            Some(rgt) => at = rgt,
                            None => break
                        }
                    } else {
                        match self.lft.get(&at) {
                            Some(lft) => at = lft,
                            None => break
                        }
                    }
                },
                None => break
            }
        }
        seen
    }

    fn insert_at(&mut self, at: u64, id: u64, key: &K) -> u64 {
        match self.key.get(&at) {
            None => {
                self.ht.insert(&id, &1);
                self.key.insert(&id, key);
                at
            },
            Some(node_key) => {
                if key.eq(&node_key) {
                    at
                } else {
                    if key.lt(&node_key) {
                        let idx = match self.lft.get(&at) {
                            Some(lft) => {
                                self.insert_at(lft, id, key)
                            },
                            None => {
                                self.insert_at(id, id, key)
                            }
                        };
                        self.lft.insert(&at, &idx);
                    } else {
                        let idx = match self.rgt.get(&at) {
                            Some(rgt) => {
                                self.insert_at(rgt, id, key)
                            },
                            None => {
                                self.insert_at(id, id, key)
                            }
                        };
                        self.rgt.insert(&at, &idx);
                    };

                    self.update_height(at);
                    self.enforce_balance(at)
                }
            }
        }
    }

    fn update_height(&mut self, at: u64) {
        let lft = self.lft.get(&at)
            .and_then(|id| self.ht.get(&id))
            .unwrap_or_default();
        let rgt = self.rgt.get(&at)
            .and_then(|id| self.ht.get(&id))
            .unwrap_or_default();

        let ht = 1 + std::cmp::max(lft, rgt);
        self.ht.insert(&at, &ht);
    }

    fn balance(&self, at: u64) -> i8 {
        let lht = self.lft.get(&at)
            .and_then(|id| self.ht.get(&id))
            .unwrap_or_default();
        let rht = self.rgt.get(&at)
            .and_then(|id| self.ht.get(&id))
            .unwrap_or_default();

        let d = lht as i64 - rht as i64;
        if d < -1 {
            -1
        } else if d > 1 {
            1
        } else {
            0
        }
    }

    fn rotate_left(&mut self, at: u64) -> u64 {
        let lft = self.lft.get(&at).unwrap();
        let lft_rgt = self.rgt.get(&lft);

        // at.L = at.L.R
        match lft_rgt {
            Some(x) => self.lft.insert(&at, &x),
            None => self.lft.remove(&at)
        };

        // at.L.R = at
        self.rgt.insert(&lft, &at);

        // at = at.L
        self.update_height(at);
        self.update_height(lft);
        lft
    }

    fn rotate_right(&mut self, at: u64) -> u64 {
        let rgt = self.rgt.get(&at).unwrap();
        let rgt_lft = self.lft.get(&rgt);

        // at.R = at.R.L
        match rgt_lft {
            Some(x) => self.rgt.insert(&at, &x),
            None => self.rgt.remove(&at)
        };

        // at.R.L = at
        self.lft.insert(&rgt, &at);

        // at = at.R
        self.update_height(at);
        self.update_height(rgt);
        rgt
    }

    fn enforce_balance(&mut self, at: u64) -> u64 {
        let balance = self.balance(at);
        if balance > 0 {
            let lft = self.lft.get(&at).unwrap();
            if self.balance(lft) < 0 {
                let rotated = self.rotate_right(lft);
                self.lft.insert(&at, &rotated);
            }
            self.rotate_left(at)
        } else if balance < 0 {
            let rgt = self.rgt.get(&at).unwrap();
            if self.balance(rgt) > 0 {
                let rotated = self.rotate_left(rgt);
                self.rgt.insert(&at, &rotated);
            }
            self.rotate_right(at)
        } else {
            at
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::test_env;

    extern crate rand;
    use self::rand::RngCore;

    fn random(n: usize) -> Vec<u32> {
        let mut rng = rand::thread_rng();
        let mut vec = Vec::with_capacity(n as usize);
        (0..n).for_each(|_| {
            vec.push(rng.next_u32());
        });
        vec
    }

    fn log2(x: f64) -> f64 {
        std::primitive::f64::log(x, 2.0f64)
    }

    fn max_tree_height(n: usize) -> u64 {
        // h <= C * log2(n + D) + B
        // where:
        // C =~ 1.440, D =~ 1.065, B =~ 0.328
        // (source: https://en.wikipedia.org/wiki/AVL_tree)
        const B: f64 = -0.328;
        const C: f64 = 1.440;
        const D: f64 = 1.065;

        let h = C * log2( n as f64 + D ) + B;
        h.ceil() as u64
    }

    #[test]
    fn test_empty() {
        test_env::setup();

        let map: TreeMap<u8, u8> = TreeMap::new(vec![b't']);
        assert_eq!(map.len(), 0);
    }

    #[test]
    fn test_insert_3_rotate_l_l() {
        test_env::setup();

        let mut map: TreeMap<i32, i32> = TreeMap::default();
        assert_eq!(map.height(), 0);

        map.insert(3, 3);
        assert_eq!(map.height(), 1);

        map.insert(2, 2);
        assert_eq!(map.height(), 2);

        map.insert(1, 1);

        let root = map.root;
        assert_eq!(root, 1);
        assert_eq!(map.key.get(&root), Some(2));
        assert_eq!(map.height(), 2);

        map.clear();
    }

    #[test]
    fn test_insert_3_rotate_r_r() {
        test_env::setup();

        let mut map: TreeMap<i32, i32> = TreeMap::default();
        assert_eq!(map.height(), 0);

        map.insert(1, 1);
        assert_eq!(map.height(), 1);

        map.insert(2, 2);
        assert_eq!(map.height(), 2);

        map.insert(3, 3);

        let root = map.root;
        assert_eq!(root, 1);
        assert_eq!(map.key.get(&root), Some(2));
        assert_eq!(map.height(), 2);

        map.clear();
    }

    #[test]
    fn test_insert_lookup_n_asc() {
        test_env::setup();

        let mut map: TreeMap<i32, i32> = TreeMap::default();

        let n: usize = 30;
        let cases = (0..2*(n as i32)).collect::<Vec<i32>>();

        let mut counter  = 0;
        for k in &cases {
            if *k % 2 == 0 {
                counter += 1;
                map.insert(*k, counter);
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

        assert!(map.height() <= max_tree_height(n));
        map.clear();
    }

    #[test]
    fn test_insert_lookup_n_desc() {
        test_env::setup();

        let mut map: TreeMap<i32, i32> = TreeMap::default();

        let n: usize = 30;
        let cases = (0..2*(n as i32)).rev().collect::<Vec<i32>>();

        let mut counter  = 0;
        for k in &cases {
            if *k % 2 == 0 {
                counter += 1;
                map.insert(*k, counter);
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

        assert!(map.height() <= max_tree_height(n));
        map.clear();
    }

    #[test]
    fn insert_n_random() {
        test_env::setup();

        for k in 1..5 {
            let mut map: TreeMap<u32, u32> = TreeMap::default();

            let n = 1 << k;
            let input: Vec<u32> = random(n);

            for x in &input {
                map.insert(*x, 42);
            }

            assert!(map.height() <= max_tree_height(n));
            map.clear();
        }
    }

    #[test]
    fn test_min() {
        test_env::setup();

        let n: usize = 30;
        let vec = random(n);

        let mut map: TreeMap<u32, u32> = TreeMap::new(vec![b't']);
        for x in vec.iter().rev() {
            map.insert(*x, 1);
        }

        assert_eq!(map.min().unwrap(), *vec.iter().min().unwrap());
        map.clear();
    }

    #[test]
    fn test_max() {
        test_env::setup();

        let n: usize = 30;
        let vec = random(n);

        let mut map: TreeMap<u32, u32> = TreeMap::new(vec![b't']);
        for x in vec.iter().rev() {
            map.insert(*x, 1);
        }

        assert_eq!(map.max().unwrap(), *vec.iter().max().unwrap());
        map.clear();
    }

    #[test]
    fn test_ceil() {
        test_env::setup();

        let mut map: TreeMap<u32, u32> = TreeMap::default();
        let vec: Vec<u32> = vec![10, 20, 30, 40, 50];

        for x in vec.iter() {
            map.insert(*x, 1);
        }

        assert_eq!(map.ceil( &5), None);
        assert_eq!(map.ceil(&10), None);
        assert_eq!(map.ceil(&11), Some(10));
        assert_eq!(map.ceil(&20), Some(10));
        assert_eq!(map.ceil(&49), Some(40));
        assert_eq!(map.ceil(&50), Some(40));
        assert_eq!(map.ceil(&51), Some(50));

        map.clear();
    }

    #[test]
    fn test_floor() {
        test_env::setup();

        let mut map: TreeMap<u32, u32> = TreeMap::default();
        let vec: Vec<u32> = vec![10, 20, 30, 40, 50];

        for x in vec.iter() {
            map.insert(*x, 1);
        }

        assert_eq!(map.floor( &5), Some(10));
        assert_eq!(map.floor(&10), Some(20));
        assert_eq!(map.floor(&11), Some(20));
        assert_eq!(map.floor(&20), Some(30));
        assert_eq!(map.floor(&49), Some(50));
        assert_eq!(map.floor(&50), None);
        assert_eq!(map.floor(&51), None);

        map.clear();
    }

    // TODO remove
    // TODO iter
    // TODO iter_rev
}
