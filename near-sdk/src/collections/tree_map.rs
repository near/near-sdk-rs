
use borsh::{BorshDeserialize, BorshSerialize};

use crate::collections::{append, append_slice, next_trie_id};
use crate::collections::UnorderedMap;

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

    pub fn height(&mut self) -> u64 {
        0 // TODO
    }

    pub fn get(&self, _key: &K) -> Option<V> {
        None // TODO
    }

    pub fn insert(&mut self, _key: K, _val: V) -> Option<V> {
        None // TODO
    }

    pub fn remove(&mut self, _key: K) -> Option<V> {
        None // TODO
    }

    pub fn min(&self) -> Option<K> {
        None // TODO
    }

    pub fn max(&self) -> Option<K> {
        None // TODO
    }

    pub fn floor(&self, _key: &K) -> Option<K> {
        None // TODO
    }

    pub fn ceil(&self, _key: &K) -> Option<K> {
        None // TODO
    }

    pub fn iter(&self, _key: &K) -> impl Iterator<Item=K> {
        // self.min() and continue with self.floor()
        std::iter::empty() // TODO
    }

    pub fn iter_rev(&self, _key: &K) -> impl Iterator<Item=K> {
        // self.max() and continue with self.ceil()
        std::iter::empty() // TODO
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::test_env;

    #[test]
    fn test_empty() {
        test_env::setup();

        let map: TreeMap<u8, u8> = TreeMap::new(vec![b't']);
        assert_eq!(map.len(), 0);
    }

    // TODO len
    // TODO height
    // TODO get
    // TODO insert
    // TODO remove
    // TODO min
    // TODO max
    // TODO floor
    // TODO ceil
    // TODO iter
    // TODO iter_rev
}