use super::{Map, TreeMap};
use crate::collections::{
    next_trie_id,
    UnorderedMap,
    RedBlackTree,
    RedBlackNodeValue,
    set::TreeSet
};
use borsh::{BorshDeserialize, BorshSerialize};
use std::ops::Bound;

#[derive(BorshSerialize, BorshDeserialize)]
pub struct OrderedMap<K, V> {
    prefix: Vec<u8>,
    tree: RedBlackTree<K>,
    map: UnorderedMap<K, V>
}

impl<K, V> OrderedMap<K, V> {
    fn new(prefix: Vec<u8>) -> Self {
        let tree_prefix = [prefix.as_slice(), "_tree".as_bytes()].concat();
        let map_prefix = [prefix.as_slice(), "_map".as_bytes()].concat();
        Self {
            prefix,
            tree: RedBlackTree::new(tree_prefix),
            map: UnorderedMap::new(map_prefix)
        }
    }
}

impl<K, V> Default for OrderedMap<K, V> {
    fn default() -> Self {
        Self::new(next_trie_id())
    }
}

impl<K, V> Map<K, V> for OrderedMap<K, V> 
where
    K: BorshSerialize + BorshDeserialize + RedBlackNodeValue + std::fmt::Debug,
    <K as RedBlackNodeValue>::OrdValue: std::fmt::Debug,
    V: BorshSerialize + BorshDeserialize + std::fmt::Debug,
{
    fn get(&self, key: &K) -> Option<V> {
        self.map.get(key)
    }

    fn remove(&mut self, key: &K) -> Option<V> {
        self.tree.remove(key.ord_value()).map(|_| self.map.remove(key)).flatten()
    }

    fn insert(&mut self, key: &K, value: &V) -> Option<V> {
        self.tree.insert(key);
        self.map.insert(key, value)
    }

    fn clear(&mut self) {
        self.tree.clear();
        self.map.clear();
    }

    fn to_vec(&self) -> std::vec::Vec<(K, V)> {
        self.tree.iter()
            .map(|key| {
                let value = self.map.get(&key).expect("value exists");
                (key, value)
            })
            .collect()
    }

    fn keys<'a>(&'a self) -> Box<dyn Iterator<Item = K> + 'a> {
        Box::new(self.tree.iter())
    }

    fn values<'a>(&'a self) -> Box<dyn Iterator<Item = V> + 'a> {
        let iter = self.tree.iter().map(move |key| self.map.get(&key).expect("value exists"));
        Box::new(iter)
    }

    fn iter<'a>(&'a self) -> Box<dyn Iterator<Item = (K, V)> + 'a> {
        let iter = self.tree.iter()
            .map(move |key| {
                let value = self.map.get(&key).expect("value exists");
                (key, value)
            });
        Box::new(iter)
    }

    fn extend<IT: IntoIterator<Item = (K, V)>>(&mut self, iter: IT) where Self: Sized {
        for entry in iter.into_iter() {
            self.map.insert(&entry.0, &entry.1);
            self.tree.insert(&entry.0);
        }
    }
}

impl<K, V> TreeMap<K, V> for OrderedMap<K, V> 
where
    K: BorshSerialize + BorshDeserialize + RedBlackNodeValue + std::fmt::Debug,
    <K as RedBlackNodeValue>::OrdValue: std::fmt::Debug,
    V: BorshSerialize + BorshDeserialize + std::fmt::Debug
{
    fn len(&self) -> u64 {
        self.tree.len()
    }

    /// Returns true if the tree contains the key, false otherwise
    fn contains_key(&self, key: &K) -> bool {
        self.tree.contains(key)
    }
    
    /// Returns the smallest stored key from the tree
    fn min(&self) -> Option<K> {
        self.tree.min()
    }
    
    /// Returns the largest stored key from the tree
    fn max(&self) -> Option<K> {
        self.tree.max()
    }

    /// Returns the smallest key that is strictly greater than key given as the parameter
    fn above(&self, key: &K) -> Option<K> {
        <RedBlackTree<K> as TreeSet<K>>::above(&self.tree, key)
    }

    /// Returns the largest key that is strictly less than key given as the parameter
    fn below(&self, key: &K) -> Option<K> {
        <RedBlackTree<K> as TreeSet<K>>::below(&self.tree, key)
    }

    /// Returns the largest key that is greater or equal to key given as the parameter
    fn ceil(&self, key: &K) -> Option<K> {
        <RedBlackTree<K> as TreeSet<K>>::ceil(&self.tree, key)
    }
    
    /// Returns the smallest key that is greater or equal to key given as the parameter
    fn floor(&self, key: &K) -> Option<K> {
        <RedBlackTree<K> as TreeSet<K>>::floor(&self.tree, key)
    }

    /// Iterates through keys in ascending order starting at key that is greater than
    /// or equal to the key supplied
    fn iter_from<'a>(&'a self, key: K) -> Box<dyn Iterator<Item = (K, V)> + 'a> {
        Box::new(
            self.tree.iter_from(key)
                .map(move |key| {
                    let value = self.map.get(&key).expect("value exists");
                    (key, value)
                })
        )
    }

    /// Iterates through keys in descending order
    fn iter_rev<'a>(&'a self) -> Box<dyn Iterator<Item = (K, V)> + 'a> {
        Box::new(
            self.tree.iter_rev()
                .map(move |key| {
                    let value = self.map.get(&key).expect("value exists");
                    (key, value)
                })
        )
    }

    /// Iterates through keys in descending order starting at key that is less than
    /// or equal to the key supplied
    fn iter_rev_from<'a>(&'a self, key: K) -> Box<dyn Iterator<Item = (K, V)> + 'a> {
        Box::new(
            self.tree.iter_rev_from(key)
                .map(move |key| {
                    let value = self.map.get(&key).expect("value exists");
                    (key, value)
                })
        )
    }

    /// Iterate over K keys in ascending order
    ///
    /// # Panics
    ///
    /// Panics if range start > end.
    /// Panics if range start == end and both bounds are Excluded.
    fn range<'a>(&'a self, r: (Bound<K>, Bound<K>)) -> Box<dyn Iterator<Item = (K, V)> + 'a> {
        Box::new(
            self.tree.range(r)
                .map(move |key| {
                    let value = self.map.get(&key).expect("value exists");
                    (key, value)
                })
        )
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[cfg(test)]
mod tests {
    use crate::collections::OrderedMap;
    use crate::{env, MockedBlockchain};
    use near_vm_logic::types::AccountId;
    use near_vm_logic::{VMContext, VMConfig};

    use crate::collections::map;

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
        set_env_config(false)
    }

    fn set_env_config(free: bool) {
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
            prepaid_gas: std::u64::MAX, //10u64.pow(19),
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
            if free { VMConfig::free() } else { Default::default() },
            Default::default(),
            vec![],
            storage,
        )));
    }

    #[test]
    pub fn test_insert() {
        set_env();
        map::tests::test_insert::<OrderedMap<u64, u64>>()
    }

    #[test]
    pub fn test_insert_remove() {
        set_env();
        map::tests::test_insert_remove::<OrderedMap<u64, u64>>()
    }

    #[test]
    pub fn test_remove_last_reinsert() {
        set_env();
        map::tests::test_remove_last_reinsert::<OrderedMap<u64, u64>>()
    }

    #[test]
    pub fn test_insert_override_remove() {
        set_env();
        map::tests::test_insert_override_remove::<OrderedMap<u64, u64>>()
    }

    #[test]
    pub fn test_get_non_existent() {
        set_env();
        map::tests::test_get_non_existent::<OrderedMap<u64, u64>>()
    }

    #[test]
    pub fn test_to_vec() {
        set_env();
        map::tests::test_to_vec::<OrderedMap<u64, u64>>()
    }

    #[test]
    pub fn test_clear() {
        set_env();
        map::tests::test_clear::<OrderedMap<u64, u64>>()
    }

    #[test]
    pub fn test_keys_values() {
        set_env();
        map::tests::test_keys_values::<OrderedMap<u64, u64>>()
    }

    #[test]
    pub fn test_iter() {
        set_env();
        map::tests::test_iter::<OrderedMap<u64, u64>>()
    }

    #[test]
    pub fn test_extend() {
        set_env();
        map::tests::test_extend::<OrderedMap<u64, u64>>()
    }

    //
    //
    //
    //

    #[test]
    fn test_empty() { set_env(); map::tests::test_empty::<OrderedMap<u8, u8>>() }
    
    #[test]
    fn test_insert_3_rotate_l_l() { set_env(); map::tests::test_insert_3_rotate_l_l::<OrderedMap<u8, u8>>() }
    
    #[test]
    fn test_insert_3_rotate_r_r() { set_env(); map::tests::test_insert_3_rotate_r_r::<OrderedMap<u8, u8>>() }
    
    #[test]
    fn test_insert_lookup_n_asc() { set_env(); map::tests::test_insert_lookup_n_asc::<OrderedMap<i32, i32>>() }
    
    #[test]
    fn test_insert_lookup_n_desc() { set_env(); map::tests::test_insert_lookup_n_desc::<OrderedMap<i32, i32>>() }
    
    #[test]
    fn insert_n_random() { set_env_config(true); map::tests::insert_n_random::<OrderedMap<u32, u32>>() }
    
    #[test]
    fn test_min() { set_env(); map::tests::test_min::<OrderedMap<u32, u32>>() }
    
    #[test]
    fn test_max() { set_env(); map::tests::test_max::<OrderedMap<u32, u32>>() }
    
    #[test]
    fn test_ceil() { set_env(); map::tests::test_ceil::<OrderedMap<u32, u32>>() }
    
    #[test]
    fn test_floor() { set_env(); map::tests::test_floor::<OrderedMap<u32, u32>>() }
    
    #[test]
    fn test_remove_1() { set_env(); map::tests::test_remove_1::<OrderedMap<u32, u32>>() }
    
    #[test]
    fn test_remove_3_desc() { set_env(); map::tests::test_remove_3_desc::<OrderedMap<u32, u32>>() }
    
    #[test]
    fn test_remove_3_asc() { set_env(); map::tests::test_remove_3_asc::<OrderedMap<u32, u32>>() }
    
    #[test]
    fn test_remove_7_regression_1() { set_env(); map::tests::test_remove_7_regression_1::<OrderedMap<u32, u32>>() }
    
    #[test]
    fn test_remove_7_regression_2() { set_env(); map::tests::test_remove_7_regression_2::<OrderedMap<u32, u32>>() }
    
    #[test]
    fn test_remove_9_regression() { set_env(); map::tests::test_remove_9_regression::<OrderedMap<u32, u32>>() }
    
    #[test]
    fn test_remove_20_regression_1() { set_env(); map::tests::test_remove_20_regression_1::<OrderedMap<u32, u32>>() }
    
    #[test]
    fn test_remove_7_regression() { set_env(); map::tests::test_remove_7_regression::<OrderedMap<u32, u32>>() }
    
    #[test]
    fn test_remove_n() { set_env(); map::tests::test_remove_n::<OrderedMap<u32, u32>>() }
    
    #[test]
    fn test_remove_root_3() { set_env(); map::tests::test_remove_root_3::<OrderedMap<u32, u32>>() }
    
    #[test]
    fn test_insert_2_remove_2_regression() { set_env(); map::tests::test_insert_2_remove_2_regression::<OrderedMap<u32, u32>>() }
    
    #[test]
    fn test_insert_n_duplicates() { set_env(); map::tests::test_insert_n_duplicates::<OrderedMap<u32, u32>>() }
    
    #[test]
    fn test_insert_2n_remove_n_random() { set_env(); map::tests::test_insert_2n_remove_n_random::<OrderedMap<u32, u32>>() }
    
    #[test]
    fn test_remove_empty() { set_env(); map::tests::test_remove_empty::<OrderedMap<u32, u32>>() }
    
    #[test]
    fn test_to_vec_empty() { set_env(); map::tests::test_to_vec_empty::<OrderedMap<u32, u32>>() }
    
    #[test]
    fn test_iter_empty() { set_env(); map::tests::test_iter_empty::<OrderedMap<u32, u32>>() }
    
    #[test]
    fn test_iter_rev() { set_env(); map::tests::test_iter_rev::<OrderedMap<u32, u32>>() }
    
    #[test]
    fn test_iter_rev_empty() { set_env(); map::tests::test_iter_rev_empty::<OrderedMap<u32, u32>>() }
    
    #[test]
    fn test_iter_from() { set_env(); map::tests::test_iter_from::<OrderedMap<u32, u32>>() }
    
    #[test]
    fn test_iter_from_empty() { set_env(); map::tests::test_iter_from_empty::<OrderedMap<u32, u32>>() }
    
    #[test]
    fn test_iter_rev_from() { set_env(); map::tests::test_iter_rev_from::<OrderedMap<u32, u32>>() }
    
    #[test]
    fn test_range() { set_env(); map::tests::test_range::<OrderedMap<u32, u32>>() }
    
    #[test]
    fn test_range_panics_same_excluded() { set_env(); map::tests::test_range_panics_same_excluded::<OrderedMap<u32, u32>>() }
    
    #[test]
    fn test_range_panics_non_overlap_incl_exlc() { set_env(); map::tests::test_range_panics_non_overlap_incl_exlc::<OrderedMap<u32, u32>>() }
    
    #[test]
    fn test_range_panics_non_overlap_excl_incl() { set_env(); map::tests::test_range_panics_non_overlap_excl_incl::<OrderedMap<u32, u32>>() }
    
    #[test]
    fn test_range_panics_non_overlap_incl_incl() { set_env(); map::tests::test_range_panics_non_overlap_incl_incl::<OrderedMap<u32, u32>>() }
    
    #[test]
    fn test_iter_rev_from_empty() { set_env(); map::tests::test_iter_rev_from_empty::<OrderedMap<u32, u32>>() }
    
    #[test]
    fn prop_tree_vs_rb() { set_env_config(true); map::tests::prop_tree_vs_rb::<OrderedMap<u32, u32>>() }
    
    #[test]
    fn prop_tree_vs_rb_range_incl_incl() { set_env_config(true); map::tests::prop_tree_vs_rb_range_incl_incl::<OrderedMap<u32, u32>>() }
    
    #[test]
    fn prop_tree_vs_rb_range_incl_excl() { set_env_config(true); map::tests::prop_tree_vs_rb_range_incl_excl::<OrderedMap<u32, u32>>() }
    
    #[test]
    fn prop_tree_vs_rb_range_excl_incl() { set_env_config(true); map::tests::prop_tree_vs_rb_range_excl_incl::<OrderedMap<u32, u32>>() }
    
    #[test]
    fn prop_tree_vs_rb_range_excl_excl() { set_env_config(true); map::tests::prop_tree_vs_rb_range_excl_excl::<OrderedMap<u32, u32>>() }
}