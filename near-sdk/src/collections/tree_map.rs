
use borsh::{BorshDeserialize, BorshSerialize};

use crate::collections::{append, next_trie_id, serialize, deserialize};
use crate::collections::UnorderedMap;
use crate::env;

/// AVL tree implementation
///
/// Runtime complexity (N = number of entries):
/// - `lookup`/`insert`/`remove`: O(log(N)) worst case
/// - `min`/`max`: O(log(N)) worst case
/// - `floor`/`ceil` (find closes key above/below): O(log(N)) worst case
/// - iterate K elements in sorted order: O(Klog(N)) worst case
///
#[derive(BorshSerialize, BorshDeserialize)]
pub struct TreeMap<K, V> {
    tree_prefix: Vec<u8>,

    len: u64,
    root: u64,                      // ID of a root node of the tree
    ht: UnorderedMap<u64, u64>,     // height of a subtree at a node
    lft: UnorderedMap<u64, u64>,    // left link of a node
    rgt: UnorderedMap<u64, u64>,    // right link of a node
    key: UnorderedMap<u64, K>,      // key stored in a node
    val: UnorderedMap<K, V>,        // value associated with key
}

impl<K, V> Default for TreeMap<K, V>
    where
        K: Ord + Copy + BorshSerialize + BorshDeserialize,
        V: Copy + BorshSerialize + BorshDeserialize,
{
    fn default() -> Self {
        Self::new(next_trie_id())
    }
}


impl<K, V> TreeMap<K, V>
    where
        K: Ord + Copy + BorshSerialize + BorshDeserialize,
        V: Copy + BorshSerialize + BorshDeserialize,
{
    pub fn new(id: Vec<u8>) -> Self {
        let h_prefix = append(&id, b'h');
        let l_prefix = append(&id, b'l');
        let r_prefix = append(&id, b'r');
        let k_prefix = append(&id, b'k');
        let v_prefix = append(&id, b'v');

        let root: u64 = env::storage_read(&id)
            .map(|raw| deserialize(&raw))
            .unwrap_or_default();
        let val = UnorderedMap::new(v_prefix);
        let len = val.len();

        Self {
            tree_prefix: id,
            root,
            len,
            ht: UnorderedMap::new(h_prefix),
            lft: UnorderedMap::new(l_prefix),
            rgt: UnorderedMap::new(r_prefix),
            key: UnorderedMap::new(k_prefix),
            val,
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
        env::storage_remove(&self.tree_prefix);
    }

    pub fn height(&self) -> u64 {
        self.ht.get(&self.root).unwrap_or_default()
    }

    pub fn contains_key(&self, key: &K) -> bool {
        self.val.get(key).is_some()
    }

    pub fn get(&self, key: &K) -> Option<V> {
        self.val.get(key)
    }

    pub fn insert(&mut self, key: K, val: V) -> Option<V> {
        if self.contains_key(&key) {
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

    pub fn remove(&mut self, key: K) -> Option<V> {
        if self.contains_key(&key) {
            let root = self.do_remove(&key);
            self.set_root(root);
            self.len -= 1;
            self.val.remove(&key)
        } else {
            // no such key, nothing to do
            None
        }
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

    pub fn iter<'a>(&'a self) -> impl Iterator<Item = (K, V)> + 'a {
        Cursor::asc(&self).into_iter()
    }

    pub fn iter_from<'a>(&'a self, key: K) -> impl Iterator<Item = (K, V)> + 'a {
        Cursor::asc_from(&self, key).into_iter()
    }

    pub fn iter_rev<'a>(&'a self) -> impl Iterator<Item = (K, V)> + 'a {
        Cursor::desc(&self).into_iter()
    }

    pub fn iter_rev_from<'a>(&'a self, key: K) -> impl Iterator<Item = (K, V)> + 'a {
        Cursor::desc_from(&self, key).into_iter()
    }

    pub fn to_vec(&self) -> Vec<(K, V)> {
        self.iter().collect()
    }

    //
    // Internal utilities
    //

    fn set_root(&mut self, root: u64) {
        env::storage_write(&self.tree_prefix, &serialize(&root));
        self.root = root;
    }

    fn min_at(&self, mut at: u64) -> Option<K> {
        loop {
            match self.lft.get(&at) {
                Some(lft) => at = lft,
                None => break
            }
        }
        self.key.get(&at)
    }

    fn max_at(&self, mut at: u64) -> Option<K> {
        loop {
            match self.rgt.get(&at) {
                Some(rgt) => at = rgt,
                None => break
            }
        }
        self.key.get(&at)
    }

    fn floor_at(&self, mut at: u64, key: &K) -> Option<K> {
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

    fn ceil_at(&self, mut at: u64, key: &K) -> Option<K> {
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

    // Calculate and save the height of a subtree at node `at`:
    // height[at] = 1 + max(height[at.L], height[at.R])
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

    // Balance = difference in heights between left and right subtrees at node `at`.
    fn get_balance(&self, at: u64) -> i64 {
        let lht = self.lft.get(&at)
            .and_then(|id| self.ht.get(&id))
            .unwrap_or_default();
        let rht = self.rgt.get(&at)
            .and_then(|id| self.ht.get(&id))
            .unwrap_or_default();

        lht as i64 - rht as i64
    }

    // Left rotation of an AVL subtree with at node `at`.
    // New root of subtree is returned, caller's responsibility is to update links accordingly.
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

    // Right rotation of an AVL subtree at node in `at`.
    // New root of subtree is returned, caller's responsibility is to update links accordingly.
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

    // Check balance at node `at` and enforce it if necessary with respective rotations.
    fn enforce_balance(&mut self, at: u64) -> u64 {
        let balance = self.get_balance(at);
        if balance > 1 {
            let lft = self.lft.get(&at).unwrap();
            if self.get_balance(lft) < 0 {
                let rotated = self.rotate_right(lft);
                self.lft.insert(&at, &rotated);
            }
            self.rotate_left(at)
        } else if balance < -1 {
            let rgt = self.rgt.get(&at).unwrap();
            if self.get_balance(rgt) > 0 {
                let rotated = self.rotate_left(rgt);
                self.rgt.insert(&at, &rotated);
            }
            self.rotate_right(at)
        } else {
            at
        }
    }

    // Returns (`id`, `parent_id`) for a node that holds the `key`.
    // For root node, root node id is returned both as `id` and as `parent_id`.
    fn lookup_at(&self, mut at: u64, key: &K) -> Option<(u64, u64)> {
        let mut p = at;
        loop {
            match self.key.get(&at) {
                Some(k) => {
                    if k.eq(key) {
                        return Some((at, p));
                    } else if k.lt(key) {
                        match self.rgt.get(&at) {
                            Some(rgt) => {
                                p = at;
                                at = rgt;
                            },
                            None => break
                        }
                    } else {
                        match self.lft.get(&at) {
                            Some(lft) => {
                                p = at;
                                at = lft;
                            },
                            None => break
                        }
                    }
                },
                None => break
            }
        }
        None
    }

    // Navigate from root to node holding `key` and backtrace back to the root -
    // enforcing balance (if imbalance takes place) along the way up to the root.
    fn check_balance(&mut self, at: u64, key: &K) -> u64 {
        match self.key.get(&at) {
            Some(k) => {
                if k.eq(key) {
                    self.update_height(at);
                    at
                } else {
                    if k.lt(key) {
                        match self.lft.get(&at) {
                            Some(l) => {
                                let id = self.check_balance(l, key);
                                self.lft.insert(&at, &id);
                            },
                            None => ()
                        }
                    } else {
                        match self.rgt.get(&at) {
                            Some(r) => {
                                let id = self.check_balance(r, key);
                                self.rgt.insert(&at, &id);
                            },
                            None => ()
                        }
                    }
                    self.update_height(at);
                    self.enforce_balance(at)
                }
            },
            None => at
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
        let root = self.root;
        // r_id - id of a node containing key of interest
        // r_p - id of an immediate parent node of r_id
        let (r_id, r_p) = match self.lookup_at(root, key) {
            Some(x) => x,
            None => return root // cannot remove a missing key, no changes to the tree needed
        };

        let lft_opt = self.lft.get(&r_id);
        let rgt_opt = self.rgt.get(&r_id);

        if lft_opt.is_none() && rgt_opt.is_none() {
            // remove leaf
            let p_key = self.key.get(&r_p).unwrap();
            if p_key.lt(key) {
                self.rgt.remove(&r_p);
            } else {
                self.lft.remove(&r_p);
            }
            self.key.remove(&r_id);
            self.ht.remove(&r_id);

            // removing node might have caused a imbalance - balance the tree up to the root,
            // starting from lowest affected key - the parent of a leaf node in this case
            self.check_balance(root, &p_key)

        } else {
            // non-leaf node, select subtree to proceed with
            let b = self.get_balance(r_id);
            if (b >= 0 && lft_opt.is_some()) || rgt_opt.is_none() {
                // proceed with left subtree
                let lft = lft_opt.unwrap();

                // k - max key from left subtree
                let k = self.max_at(lft).unwrap();

                // n - id of a node that holds key k, p - id of immediate parent of n
                let (n, p) = self.lookup_at(r_id, &k).unwrap();

                self.key.insert(&r_id, &k);
                self.key.remove(&n);
                self.ht.remove(&n);

                if self.rgt.get(&p).map(|id| id == n).unwrap_or_default() {
                    // n is on right link of p
                    match self.lft.get(&n) {
                        Some(l) => {
                            self.rgt.insert(&p, &l);
                            self.lft.remove(&n);
                        },
                        None => {
                            self.rgt.remove(&p);
                        }
                    };
                } else {
                    // n is on left link of p
                    match self.lft.get(&n) {
                        Some(l) => {
                            self.lft.insert(&p, &l);
                            self.lft.remove(&n);
                        },
                        None => {
                            self.lft.remove(&p);
                        }
                    }
                }

                // removing node might have caused a imbalance - balance the tree up to the root,
                // starting from the lowest affected key (max key from left subtree in this case)
                self.check_balance(root, &k)

            } else if (b < 0 && rgt_opt.is_some()) || lft_opt.is_none() {
                // proceed with right subtree
                let rgt = rgt_opt.unwrap();

                // k - min key from right subtree
                let k = self.min_at(rgt).unwrap();

                // n - id of a node that holds key k, p - id of an immediate parent of n
                let (n, p) = self.lookup_at(r_id, &k).unwrap();

                self.key.insert(&r_id, &k);
                self.key.remove(&n);
                self.ht.remove(&n);

                if self.lft.get(&p).map(|id| id == n).unwrap_or_default() {
                    // n is on left link of p
                    match self.rgt.get(&n) {
                        Some(r) => {
                            self.lft.insert(&p, &r);
                            self.rgt.remove(&n);
                        },
                        None => {
                            self.lft.remove(&p);
                        }
                    }
                } else {
                    // n is on right link of p
                    match self.rgt.get(&n) {
                        Some(r) => {
                            self.rgt.insert(&p, &r);
                            self.rgt.remove(&n);
                        },
                        None => {
                            self.rgt.remove(&p);
                        }
                    }
                }

                // removing node might have caused a imbalance - balance the tree up to the root,
                // starting from the lowest affected key (min key from right subtree in this case)
                self.check_balance(root, &k)

            } else {
                root
            }
        }
    }
}

impl<'a, K, V> IntoIterator for &'a TreeMap<K, V>
    where
        K: Ord + Copy + BorshSerialize + BorshDeserialize,
        V: Copy + BorshSerialize + BorshDeserialize,
{
    type Item = (K, V);
    type IntoIter = Cursor<'a, K, V>;

    fn into_iter(self) -> Self::IntoIter {
        Cursor::asc(self)
    }
}

impl<K, V> Iterator for Cursor<'_, K, V>
    where
        K: Ord + Copy + BorshSerialize + BorshDeserialize,
        V: Copy + BorshSerialize + BorshDeserialize,
{
    type Item = (K, V);

    fn next(&mut self) -> Option<Self::Item> {
        let key = if self.asc {
            match self.key {
                Some(k) => self.map.floor(&k),
                None => self.map.min()
            }
        } else {
            match self.key {
                Some(k) => self.map.ceil(&k),
                None => self.map.max()
            }
        };
        self.key = key;

        key.and_then(|k| self.map.get(&k).map(|v| (k, v)))
    }
}

pub struct Cursor<'a, K, V> {
    asc: bool,
    key: Option<K>,
    map: &'a TreeMap<K, V>
}

impl<'a, K, V> Cursor<'a, K, V>
    where
        K: Ord + BorshSerialize + BorshDeserialize,
        V: BorshSerialize + BorshDeserialize,
{
    fn asc(map: &'a TreeMap<K, V>) -> Self {
        Self {
            asc: true,
            key: None,
            map
        }
    }

    fn asc_from(map: &'a TreeMap<K, V>, key: K) -> Self {
        Self {
            asc: true,
            key: Some(key),
            map
        }
    }

    fn desc(map: &'a TreeMap<K, V>) -> Self {
        Self {
            asc: false,
            key: None,
            map
        }
    }

    fn desc_from(map: &'a TreeMap<K, V>, key: K) -> Self {
        Self {
            asc: false,
            key: Some(key),
            map
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

    fn random(n: u64) -> Vec<u32> {
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

    fn max_tree_height(n: u64) -> u64 {
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
        assert_eq!(map.height(), 0);
        assert_eq!(map.get(&42), None);
        assert!(!map.contains_key(&42));
        assert_eq!(map.min(), None);
        assert_eq!(map.max(), None);
        assert_eq!(map.ceil(&42), None);
        assert_eq!(map.floor(&42), None);
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

        let n: u64 = 30;
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

        let n: u64 = 30;
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

        let n: u64 = 30;
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

        let n: u64 = 30;
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

    #[test]
    fn test_remove_1() {
        test_env::setup();

        let mut map: TreeMap<u32, u32> = TreeMap::default();
        map.insert(1, 1);
        assert_eq!(map.get(&1), Some(1));
        map.remove(1);
        assert_eq!(map.get(&1), None);
        assert_eq!(map.key.len(), 0);
        assert_eq!(map.ht.len(), 0);
        map.clear();
    }

    #[test]
    fn test_remove_3_desc() {
        test_env::setup();

        let vec: Vec<u32> = vec![3, 2, 1];
        let mut map: TreeMap<u32, u32> = TreeMap::default();

        for x in &vec {
            assert_eq!(map.get(x), None);
            map.insert(*x, 1);
            assert_eq!(map.get(x), Some(1));
        }

        for x in &vec {
            assert_eq!(map.get(x), Some(1));
            map.remove(*x);
            assert_eq!(map.get(x), None);
        }
        map.clear();
    }

    #[test]
    fn test_remove_3_asc() {
        test_env::setup();

        let vec: Vec<u32> = vec![1, 2, 3];
        let mut map: TreeMap<u32, u32> = TreeMap::default();

        for x in &vec {
            assert_eq!(map.get(x), None);
            map.insert(*x, 1);
            assert_eq!(map.get(x), Some(1));
        }

        for x in &vec {
            assert_eq!(map.get(x), Some(1));
            map.remove(*x);
            assert_eq!(map.get(x), None);
        }
        map.clear();
    }

    #[test]
    fn test_remove_7_regression_1() {
        test_env::setup();

        let vec: Vec<u32> = vec![2104297040, 552624607, 4269683389, 3382615941,
                                 155419892, 4102023417, 1795725075];
        let mut map: TreeMap<u32, u32> = TreeMap::default();

        for x in &vec {
            assert_eq!(map.get(x), None);
            map.insert(*x, 1);
            assert_eq!(map.get(x), Some(1));
        }

        for x in &vec {
            assert_eq!(map.get(x), Some(1));
            map.remove(*x);
            assert_eq!(map.get(x), None);
        }
        map.clear();
    }

    #[test]
    fn test_remove_7_regression_2() {
        test_env::setup();

        let vec: Vec<u32> = vec![700623085, 87488544, 1500140781, 1111706290,
                                 3187278102, 4042663151, 3731533080];
        let mut map: TreeMap<u32, u32> = TreeMap::default();

        for x in &vec {
            assert_eq!(map.get(x), None);
            map.insert(*x, 1);
            assert_eq!(map.get(x), Some(1));
        }

        for x in &vec {
            assert_eq!(map.get(x), Some(1));
            map.remove(*x);
            assert_eq!(map.get(x), None);
        }
        map.clear();
    }

    #[test]
    fn test_remove_9_regression() {
        test_env::setup();

        let vec: Vec<u32> = vec![1186903464, 506371929, 1738679820, 1883936615, 1815331350,
                                 1512669683, 3581743264, 1396738166, 1902061760];
        let mut map: TreeMap<u32, u32> = TreeMap::default();

        for x in &vec {
            assert_eq!(map.get(x), None);
            map.insert(*x, 1);
            assert_eq!(map.get(x), Some(1));
        }

        for x in &vec {
            assert_eq!(map.get(x), Some(1));
            map.remove(*x);
            assert_eq!(map.get(x), None);
        }
        map.clear();
    }

    #[test]
    fn test_remove_20_regression() {
        test_env::setup();

        let vec: Vec<u32> = vec![552517392, 3638992158, 1015727752, 2500937532, 638716734,
                                 586360620, 2476692174, 1425948996, 3608478547, 757735878,
                                 2709959928, 2092169539, 3620770200, 783020918, 1986928932,
                                 200210441, 1972255302, 533239929, 497054557, 2137924638];
        let mut map: TreeMap<u32, u32> = TreeMap::default();

        for x in &vec {
            assert_eq!(map.get(x), None);
            map.insert(*x, 1);
            assert_eq!(map.get(x), Some(1));
        }

        for x in &vec {
            assert_eq!(map.get(x), Some(1));
            map.remove(*x);
            assert_eq!(map.get(x), None);
        }
        map.clear();
    }

    #[test]
    fn test_remove_n() {
        test_env::setup();

        let n: u64 = 20;
        let vec = random(n);

        let mut map: TreeMap<u32, u32> = TreeMap::default();
        for x in &vec {
            map.insert(*x, 1);
        }

        assert_eq!(map.len(), n);

        for x in &vec {
            assert_eq!(map.get(x), Some(1));
            map.remove(*x);
            assert_eq!(map.get(x), None);
        }

        assert_eq!(map.len(), 0);

        assert_eq!(map.key.len(), 0);
        assert_eq!(map.val.len(), 0);
        assert_eq!(map.ht.len(), 0);
        assert_eq!(map.lft.len(), 0);
        assert_eq!(map.rgt.len(), 0);
        map.clear();
    }

    #[test]
    fn test_remove_root_3() {
        test_env::setup();

        let mut map: TreeMap<u32, u32> = TreeMap::default();
        map.insert(2, 1);
        map.insert(3, 1);
        map.insert(1, 1);
        map.insert(4, 1);

        map.remove(2);

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

        let mut map: TreeMap<u32, u32> = TreeMap::default();
        map.insert(ins[0], 1);
        map.insert(ins[1], 1);

        map.remove(rem[0]);
        map.remove(rem[1]);

        let h = map.height();
        let h_max = max_tree_height(map.len());
        assert!(h <= h_max, "h={} h_max={}", h, h_max);
        map.clear();
    }

    #[test]
    fn test_insert_n_duplicates() {
        test_env::setup();
        let mut map: TreeMap<u32, u32> = TreeMap::default();

        for x in 0..30 {
            map.insert(x, x);
            map.insert(42, x);
        }

        assert_eq!(map.get(&42), Some(29));
        assert_eq!(map.len(), 31);
        assert_eq!(map.key.len(), 31);
        assert_eq!(map.ht.len(), 31);

        map.clear();
    }

    #[test]
    fn test_insert_2n_remove_n_random() {
        test_env::setup();

        for k in 1..4 {
            let mut map: TreeMap<u32, u32> = TreeMap::default();

            let n = 1 << k;
            let ins: Vec<u32> = random(n);
            let rem: Vec<u32> = random(n);

            for x in &ins {
                map.insert(*x, 42);
            }

            for x in &rem {
                map.insert(*x, 42);
            }

            for x in &rem {
                map.remove(*x);
            }

            assert_eq!(map.len(), n);

            let h = map.height();
            let h_max = max_tree_height(n);
            assert!(h <= h_max, "[n={}] tree is too high: {} (max is {}).", n, h, h_max);

            map.clear();
        }
    }

    #[test]
    fn test_remove_empty() {
        test_env::setup();
        let mut map: TreeMap<u32, u32> = TreeMap::default();
        assert_eq!(map.remove(1), None);
    }

    #[test]
    fn test_to_vec() {
        test_env::setup();
        let mut map: TreeMap<u32, u32> = TreeMap::default();
        map.insert(1, 41);
        map.insert(2, 42);
        map.insert(3, 43);

        assert_eq!(map.to_vec(), vec![(1, 41), (2, 42), (3, 43)]);
        map.clear();
    }

    #[test]
    fn test_to_vec_empty() {
        test_env::setup();
        let map: TreeMap<u32, u32> = TreeMap::default();
        assert!(map.to_vec().is_empty());
    }

    #[test]
    fn test_iter() {
        test_env::setup();
        let mut map: TreeMap<u32, u32> = TreeMap::default();
        map.insert(1, 41);
        map.insert(2, 42);
        map.insert(3, 43);

        assert_eq!(map.iter().collect::<Vec<(u32, u32)>>(), vec![(1, 41), (2, 42), (3, 43)]);
        map.clear();
    }

    #[test]
    fn test_iter_empty() {
        test_env::setup();
        let map: TreeMap<u32, u32> = TreeMap::default();
        assert!(map.iter().collect::<Vec<(u32, u32)>>().is_empty());
    }

    #[test]
    fn test_iter_rev() {
        test_env::setup();
        let mut map: TreeMap<u32, u32> = TreeMap::default();
        map.insert(1, 41);
        map.insert(2, 42);
        map.insert(3, 43);

        assert_eq!(map.iter_rev().collect::<Vec<(u32, u32)>>(), vec![(3, 43), (2, 42), (1, 41)]);
        map.clear();
    }

    #[test]
    fn test_iter_rev_empty() {
        test_env::setup();
        let map: TreeMap<u32, u32> = TreeMap::default();
        assert!(map.iter_rev().collect::<Vec<(u32, u32)>>().is_empty());
    }

    #[test]
    fn test_iter_from() {
        test_env::setup();
        let mut map: TreeMap<u32, u32> = TreeMap::default();

        let one: Vec<u32> = vec![10, 20, 30, 40, 50];
        let two: Vec<u32> = vec![45, 35, 25, 15, 5];

        for x in &one {
            map.insert(*x, 42);
        }

        for x in &two {
            map.insert(*x, 42);
        }

        assert_eq!(
            map.iter_from(29).collect::<Vec<(u32, u32)>>(),
            vec![(30, 42), (35, 42), (40, 42), (45, 42), (50, 42)]);

        assert_eq!(
            map.iter_from(30).collect::<Vec<(u32, u32)>>(),
            vec![(35, 42), (40, 42), (45, 42), (50, 42)]);

        assert_eq!(
            map.iter_from(31).collect::<Vec<(u32, u32)>>(),
            vec![(35, 42), (40, 42), (45, 42), (50, 42)]);
        map.clear();
    }

    #[test]
    fn test_iter_from_empty() {
        test_env::setup();
        let map: TreeMap<u32, u32> = TreeMap::default();
        assert!(map.iter_from(42).collect::<Vec<(u32, u32)>>().is_empty());
    }

    #[test]
    fn test_iter_rev_from() {
        test_env::setup();
        let mut map: TreeMap<u32, u32> = TreeMap::default();

        let one: Vec<u32> = vec![10, 20, 30, 40, 50];
        let two: Vec<u32> = vec![45, 35, 25, 15, 5];

        for x in &one {
            map.insert(*x, 42);
        }

        for x in &two {
            map.insert(*x, 42);
        }

        assert_eq!(
            map.iter_rev_from(29).collect::<Vec<(u32, u32)>>(),
            vec![(25, 42), (20, 42), (15, 42), (10, 42), (5, 42)]);

        assert_eq!(
            map.iter_rev_from(30).collect::<Vec<(u32, u32)>>(),
            vec![(25, 42), (20, 42), (15, 42), (10, 42), (5, 42)]);

        assert_eq!(
            map.iter_rev_from(31).collect::<Vec<(u32, u32)>>(),
            vec![(30, 42), (25, 42), (20, 42), (15, 42), (10, 42), (5, 42)]);
        map.clear();
    }

    #[test]
    fn test_iter_rev_from_empty() {
        test_env::setup();
        let map: TreeMap<u32, u32> = TreeMap::default();
        assert!(map.iter_rev_from(42).collect::<Vec<(u32, u32)>>().is_empty());
    }
}
