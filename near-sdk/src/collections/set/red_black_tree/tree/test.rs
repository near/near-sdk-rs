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