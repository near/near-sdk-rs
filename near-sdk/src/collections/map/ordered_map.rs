use super::Map;

use crate::collections::{
    Vector,
    RedBlackTree,
    RedBlackNodeValue
};
use crate::env;
use borsh::{BorshDeserialize, BorshSerialize};

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct OrderedMapEntry<K, V> {
    key: K,
    value: V
}

impl<K, V> Into<(K, V)> for OrderedMapEntry<K, V> {
    fn into(self) -> (K, V) {
        (self.key, self.value)
    }
}

impl<K, V> From<(K, V)> for OrderedMapEntry<K, V> {
    fn from(other: (K, V)) -> Self {
        Self {
            key: other.0,
            value: other.1
        }
    }
}

impl<K, V> RedBlackNodeValue for OrderedMapEntry<K, V> 
where
    K: Ord
{
    type OrdValue = K;

    fn ord_value(&self) -> &Self::OrdValue {
        &self.key
    }
}

impl<K, V> Ord for OrderedMapEntry<K, V> 
where
    Self: RedBlackNodeValue
{
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.ord_value().cmp(&other.ord_value())
    }
}

impl<K, V> PartialOrd for OrderedMapEntry<K, V> 
where
    Self: RedBlackNodeValue
{
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.ord_value().cmp(&other.ord_value()))
    }
}

impl<K, V> PartialEq for OrderedMapEntry<K, V> 
where
    Self: RedBlackNodeValue
{
    fn eq(&self, other: &Self) -> bool {
        self.ord_value() == other.ord_value()
    }
}

impl<K, V> Eq for OrderedMapEntry<K, V> 
where
    Self: RedBlackNodeValue
{}

#[derive(BorshSerialize, BorshDeserialize)]
pub struct OrderedMap<K, V> {
    tree: RedBlackTree<OrderedMapEntry<K, V>>
}

// impl<K, V> OrderedMap<K, V> {
//     fn new() -> Self {
//         Self {
//             tree: RedBlackTree::new(next_trie_id())
//         }
//     }
// }

impl<K, V> Default for OrderedMap<K, V> {
    fn default() -> Self {
        Self {
            tree: RedBlackTree::default()
        }
    }
}

impl<K, V> Map<K, V> for OrderedMap<K, V> 
where
    K: BorshSerialize + BorshDeserialize + Clone + Ord + std::fmt::Debug,
    V: BorshSerialize + BorshDeserialize + Clone + std::fmt::Debug,
{
    fn get(&self, key: &K) -> Option<V> {
        self.tree.get(key).map(|entry| entry.value)
    }

    fn remove(&mut self, key: &K) -> Option<V> {
        self.tree.remove(key).map(|entry| entry.value)
    }

    fn insert(&mut self, key: &K, value: &V) -> Option<V> {
        self.tree.add(OrderedMapEntry { key: key.clone(), value: value.clone() }).map(|entry| entry.value)
    }

    fn clear(&mut self) {
        // FIXME make this efficient
        let v: Vec<OrderedMapEntry<K, V>> = self.tree.iter().collect();
        for entry in v.iter() {
            self.tree.remove(&entry.key);
        }
    }

    fn to_vec(&self) -> std::vec::Vec<(K, V)> {
        self.tree.iter().map(|entry| entry.into()).collect()
    }

    fn keys<'a>(&'a self) -> Box<dyn Iterator<Item = K> + 'a> {
        Box::new(self.tree.iter().map(|entry| entry.key))
    }

    fn values<'a>(&'a self) -> Box<dyn Iterator<Item = V> + 'a> {
        Box::new(self.tree.iter().map(|entry| entry.value))
    }

    fn iter<'a>(&'a self) -> Box<dyn Iterator<Item = (K, V)> + 'a> {
        Box::new(self.tree.iter().map(|entry| entry.into()))
    }

    fn extend<IT: IntoIterator<Item = (K, V)>>(&mut self, iter: IT) where Self: Sized {
        for entry in iter.into_iter().map(|entry| OrderedMapEntry::from(entry)) {
            self.tree.add(entry);
        }
    }

    // fn keys_as_vector(&self) -> &Vector<K> {
    //     Self::keys_as_vector(self)
    // }

    // fn values_as_vector(&self) -> &Vector<V> {
    //     Self::values_as_vector(self)
    // }
}

#[cfg(not(target_arch = "wasm32"))]
#[cfg(test)]
mod tests {
    use crate::collections::OrderedMap;
    use crate::{env, MockedBlockchain};
    use near_vm_logic::types::AccountId;
    use near_vm_logic::VMContext;
    use rand::seq::SliceRandom;
    use rand::{Rng, SeedableRng};
    use std::collections::{HashMap, HashSet};
    use std::iter::FromIterator;

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
            prepaid_gas: 10u64.pow(19),
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
}