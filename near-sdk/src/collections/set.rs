//! A set implemented on a trie. Unlike `std::collections::HashSet` the elements in this set are not
//! hashed but are instead serialized.
use crate::collections::{next_trie_id, Vector};
use crate::env;
use borsh::{BorshDeserialize, BorshSerialize};
use std::mem::size_of;

const ERR_INCONSISTENT_STATE: &[u8] = b"The collection is an inconsistent state. Did previous smart contract execution terminate unexpectedly?";
const ERR_ELEMENT_SERIALIZATION: &[u8] = b"Cannot serialize element with Borsh";

/// An iterable implementation of a set that stores its content directly on the trie.
#[derive(BorshSerialize, BorshDeserialize)]
pub struct Set<T> {
    element_index_prefix: Vec<u8>,
    elements: Vector<T>,
}

impl<T> Default for Set<T> {
    fn default() -> Self {
        Self::new(next_trie_id())
    }
}

impl<T> Set<T> {
    /// Returns the number of elements in the set, also referred to as its size.
    pub fn len(&self) -> u64 {
        self.elements.len()
    }

    /// Create new map with zero elements. Use `id` as a unique identifier.
    pub fn new(id: Vec<u8>) -> Self {
        let mut element_index_prefix = Vec::with_capacity(id.len() + 1);
        element_index_prefix.extend(&id);
        element_index_prefix.push(b'i');

        let mut elements_prefix = Vec::with_capacity(id.len() + 1);
        elements_prefix.extend(&id);
        elements_prefix.push(b'e');

        Self { element_index_prefix, elements: Vector::new(elements_prefix) }
    }

    fn serialize_index(index: u64) -> [u8; size_of::<u64>()] {
        index.to_le_bytes()
    }

    fn deserialize_index(raw_index: &[u8]) -> u64 {
        let mut result = [0u8; size_of::<u64>()];
        result.copy_from_slice(raw_index);
        u64::from_le_bytes(result)
    }

    fn raw_element_to_index_lookup(&self, element_raw: &[u8]) -> Vec<u8> {
        let mut res = Vec::with_capacity(self.element_index_prefix.len() + element_raw.len());
        res.extend_from_slice(&self.element_index_prefix);
        res.extend_from_slice(&element_raw);
        res
    }

    /// Returns true if the set contains a serialized element.
    fn contains_raw(&self, element_raw: &[u8]) -> bool {
        let index_lookup = self.raw_element_to_index_lookup(element_raw);
        env::storage_has_key(&index_lookup)
    }

    /// Adds a value to the set.
    /// If the set did not have this value present, `true` is returned.
    /// If the set did have this value present, `false` is returned.
    pub fn insert_raw(&mut self, element_raw: &[u8]) -> bool {
        let index_lookup = self.raw_element_to_index_lookup(element_raw);
        match env::storage_read(&index_lookup) {
            Some(_index_raw) => false,
            None => {
                // The element does not exist yet.
                let next_index = self.len();
                let next_index_raw = Self::serialize_index(next_index);
                env::storage_write(&index_lookup, &next_index_raw);
                self.elements.push_raw(element_raw);
                true
            }
        }
    }

    /// Removes a value from the set. Returns whether the value was present in the set.
    pub fn remove_raw(&mut self, element_raw: &[u8]) -> bool {
        let index_lookup = self.raw_element_to_index_lookup(element_raw);
        match env::storage_read(&index_lookup) {
            Some(index_raw) => {
                if self.len() == 1 {
                    // If there is only one element then swap remove simply removes it without
                    // swapping with the last element.
                    env::storage_remove(&index_lookup);
                } else {
                    // If there is more than one element then swap remove swaps it with the last
                    // element.
                    let last_element_raw = match self.elements.get_raw(self.len() - 1) {
                        Some(x) => x,
                        None => env::panic(ERR_INCONSISTENT_STATE),
                    };
                    env::storage_remove(&index_lookup);
                    // If the removed element was the last element from keys, then we don't need to
                    // reinsert the lookup back.
                    if last_element_raw != element_raw {
                        let last_lookup_element =
                            self.raw_element_to_index_lookup(&last_element_raw);
                        env::storage_write(&last_lookup_element, &index_raw);
                    }
                }
                let index = Self::deserialize_index(&index_raw);
                self.elements.swap_remove_raw(index);
                true
            }
            None => false,
        }
    }
}

impl<T> Set<T>
where
    T: BorshSerialize + BorshDeserialize,
{
    fn serialize_element(element: &T) -> Vec<u8> {
        match element.try_to_vec() {
            Ok(x) => x,
            Err(_) => env::panic(ERR_ELEMENT_SERIALIZATION),
        }
    }

    /// Returns true if the set contains an element.
    pub fn contains(&self, element: &T) -> bool {
        self.contains_raw(&Self::serialize_element(element))
    }

    /// Removes a value from the set. Returns whether the value was present in the set.
    pub fn remove(&mut self, element: &T) -> bool {
        self.remove_raw(&Self::serialize_element(element))
    }

    /// Adds a value to the set.
    /// If the set did not have this value present, `true` is returned.
    /// If the set did have this value present, `false` is returned.
    pub fn insert(&mut self, element: &T) -> bool {
        self.insert_raw(&Self::serialize_element(element))
    }

    /// Clears the map, removing all elements.
    pub fn clear(&mut self) {
        for raw_element in self.elements.iter_raw() {
            let index_lookup = self.raw_element_to_index_lookup(&raw_element);
            env::storage_remove(&index_lookup);
        }
        self.elements.clear();
    }

    /// Copies elements into an `std::vec::Vec`.
    pub fn to_vec(&self) -> std::vec::Vec<T> {
        self.iter().collect()
    }

    /// Iterate over deserialized elements.
    pub fn iter<'a>(&'a self) -> impl Iterator<Item = T> + 'a {
        self.elements.iter()
    }

    pub fn extend<IT: IntoIterator<Item = T>>(&mut self, iter: IT) {
        for el in iter {
            self.insert(&el);
        }
    }

    /// Returns a view of elements as a vector.
    /// It's sometimes useful to have random access to the elements.
    pub fn as_vector(&self) -> &Vector<T> {
        &self.elements
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[cfg(test)]
mod tests {
    use crate::collections::Set;
    use crate::{env, MockedBlockchain};
    use near_vm_logic::types::AccountId;
    use near_vm_logic::VMContext;
    use rand::seq::SliceRandom;
    use rand::{Rng, SeedableRng};
    use std::collections::HashSet;
    use std::iter::FromIterator;

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
    pub fn test_insert() {
        set_env();
        let mut set = Set::default();
        let mut rng = rand_xorshift::XorShiftRng::seed_from_u64(0);
        for _ in 0..1000 {
            let key = rng.gen::<u64>();
            set.insert(&key);
        }
    }

    #[test]
    pub fn test_insert_remove() {
        set_env();
        let mut set = Set::default();
        let mut rng = rand_xorshift::XorShiftRng::seed_from_u64(1);
        let mut keys = vec![];
        for _ in 0..100 {
            let key = rng.gen::<u64>();
            keys.push(key);
            set.insert(&key);
        }
        keys.shuffle(&mut rng);
        for key in keys {
            assert!(set.remove(&key));
        }
    }

    #[test]
    pub fn test_remove_last_reinsert() {
        set_env();
        let mut set = Set::default();
        let key1 = 1u64;
        set.insert(&key1);
        let key2 = 2u64;
        set.insert(&key2);

        let actual = set.remove(&key2);
        assert!(actual);

        let actual_reinsert = set.insert(&key2);
        assert!(actual_reinsert);
    }

    #[test]
    pub fn test_insert_override_remove() {
        set_env();
        let mut set = Set::default();
        let mut rng = rand_xorshift::XorShiftRng::seed_from_u64(2);
        let mut keys = vec![];
        for _ in 0..100 {
            let key = rng.gen::<u64>();
            keys.push(key);
            set.insert(&key);
        }
        keys.shuffle(&mut rng);
        for key in &keys {
            assert!(!set.insert(key));
        }
        keys.shuffle(&mut rng);
        for key in keys {
            assert!(set.remove(&key));
        }
    }

    #[test]
    pub fn test_contains_non_existent() {
        set_env();
        let mut set = Set::default();
        let mut rng = rand_xorshift::XorShiftRng::seed_from_u64(3);
        let mut set_tmp = HashSet::new();
        for _ in 0..1000 {
            let key = rng.gen::<u64>() % 20_000;
            set_tmp.insert(key);
            set.insert(&key);
        }
        for _ in 0..1000 {
            let key = rng.gen::<u64>() % 20_000;
            assert_eq!(set.contains(&key), set_tmp.contains(&key));
        }
    }

    #[test]
    pub fn test_to_vec() {
        set_env();
        let mut set = Set::default();
        let mut rng = rand_xorshift::XorShiftRng::seed_from_u64(4);
        let mut keys = HashSet::new();
        for _ in 0..1000 {
            let key = rng.gen::<u64>();
            keys.insert(key);
            set.insert(&key);
        }
        let actual = HashSet::from_iter(set.to_vec());
        assert_eq!(actual, keys);
    }

    #[test]
    pub fn test_clear() {
        set_env();
        let mut set = Set::default();
        let mut rng = rand_xorshift::XorShiftRng::seed_from_u64(5);
        for _ in 0..10 {
            for _ in 0..=(rng.gen::<u64>() % 20 + 1) {
                let key = rng.gen::<u64>();
                set.insert(&key);
            }
            assert!(!set.to_vec().is_empty());
            set.clear();
            assert!(set.to_vec().is_empty());
        }
    }

    #[test]
    pub fn test_iter() {
        set_env();
        let mut set = Set::default();
        let mut rng = rand_xorshift::XorShiftRng::seed_from_u64(4);
        let mut keys = HashSet::new();
        for _ in 0..1000 {
            let key = rng.gen::<u64>();
            keys.insert(key);
            set.insert(&key);
        }
        let actual: HashSet<u64> = HashSet::from_iter(set.iter());
        assert_eq!(actual, keys);
    }

    #[test]
    pub fn test_extend() {
        set_env();
        let mut set = Set::default();
        let mut rng = rand_xorshift::XorShiftRng::seed_from_u64(4);
        let mut keys = HashSet::new();
        for _ in 0..100 {
            let key = rng.gen::<u64>();
            keys.insert(key);
            set.insert(&key);
        }
        for _ in 0..100 {
            let mut tmp = vec![];
            for _ in 0..=(rng.gen::<u64>() % 20 + 1) {
                let key = rng.gen::<u64>();
                tmp.push(key);
            }
            keys.extend(tmp.iter().cloned());
            set.extend(tmp.iter().cloned());
        }

        let actual: HashSet<u64> = HashSet::from_iter(set.iter());
        assert_eq!(actual, keys);
    }
}
