//! A set implemented on a trie. Unlike `std::collections::HashSet` the elements in this set are not
//! hashed but are instead serialized.
use crate::collections::next_trie_id;
use crate::env;
use borsh::{BorshDeserialize, BorshSerialize};
use near_vm_logic::types::IteratorIndex;
use std::marker::PhantomData;

#[derive(BorshSerialize, BorshDeserialize)]
pub struct Set<T> {
    len: u64,
    prefix: Vec<u8>,
    #[borsh_skip]
    element: PhantomData<T>,
}

impl<T> Set<T> {
    /// Returns the number of elements in the set, also referred to as its 'size'.
    pub fn len(&self) -> u64 {
        self.len
    }
}

impl<T> Default for Set<T> {
    fn default() -> Self {
        Self::new(next_trie_id())
    }
}

impl<T> Set<T> {
    /// Create new set with zero elements. Use `id` as a unique identifier.
    pub fn new(id: Vec<u8>) -> Self {
        Self { len: 0, prefix: id, element: PhantomData }
    }
}

impl<T> Set<T>
where
    T: BorshSerialize + BorshDeserialize,
{
    /// Serializes element into an array of bytes.
    fn serialize_element(&self, element: &T) -> Vec<u8> {
        let mut res = self.prefix.clone();
        let data = element.try_to_vec().expect("Element should be serializable with Borsh.");
        res.extend(data);
        res
    }

    /// Deserializes element, taking prefix into account.
    fn deserialize_element(prefix: &[u8], raw_element: &[u8]) -> T {
        let element = &raw_element[prefix.len()..];
        T::try_from_slice(element).expect("Element should be deserializable with Borsh.")
    }

    /// An iterator visiting all elements. The iterator element type is `T`.
    pub fn iter<'a>(&'a self) -> impl Iterator<Item = T> + 'a {
        let prefix = self.prefix.clone();
        self.raw_elements().map(move |k| Self::deserialize_element(&prefix, &k))
    }

    /// Returns `true` if the set contains a value
    pub fn contains(&self, element: &T) -> bool {
        let raw_element = self.serialize_element(element);
        env::storage_read(&raw_element).is_some()
    }

    /// Removes an element from the set, returning `true` if the element was present.
    pub fn remove(&mut self, element: &T) -> bool {
        let raw_element = self.serialize_element(element);
        if env::storage_remove(&raw_element) {
            self.len -= 1;
            true
        } else {
            false
        }
    }

    /// Inserts an element into the set. If element was already present returns `true`.
    pub fn insert(&mut self, element: &T) -> bool {
        let raw_element = self.serialize_element(element);
        if env::storage_write(&raw_element, &[]) {
            true
        } else {
            self.len += 1;
            false
        }
    }

    /// Copies elements into an `std::vec::Vec`.
    pub fn to_vec(&self) -> std::vec::Vec<T> {
        self.iter().collect()
    }

    /// Raw serialized elements.
    fn raw_elements(&self) -> IntoSetRawElements {
        let iterator_id = env::storage_iter_prefix(&self.prefix);
        IntoSetRawElements { iterator_id }
    }
    /// Clears the set, removing all elements.
    pub fn clear(&mut self) {
        let elements: Vec<Vec<u8>> = self.raw_elements().collect();
        for element in elements {
            env::storage_remove(&element);
        }
        self.len = 0;
    }

    pub fn extend<IT: IntoIterator<Item = T>>(&mut self, iter: IT) {
        for el in iter {
            let element = self.serialize_element(&el);
            if !env::storage_write(&element, &[]) {
                self.len += 1;
            }
        }
    }
}

/// Non-consuming iterator over raw serialized elements of `Set<T>`.
pub struct IntoSetRawElements {
    iterator_id: IteratorIndex,
}

impl Iterator for IntoSetRawElements {
    type Item = Vec<u8>;

    fn next(&mut self) -> Option<Self::Item> {
        if env::storage_iter_next(self.iterator_id) {
            env::storage_iter_key_read()
        } else {
            None
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[cfg(test)]
mod tests {
    use near_vm_logic::types::AccountId;
    use near_vm_logic::VMContext;
    use crate::{env, MockedBlockchain};
    use crate::collections::Set;
    use rand::{SeedableRng, Rng};
    use rand::seq::SliceRandom;
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
            prepaid_gas: 10u64.pow(9),
            random_seed: vec![0, 1, 2],
            is_view: false,
            output_data_receivers: vec![],
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
        for _ in 0..10_000 {
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
        for _ in 0..10_000 {
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
    pub fn test_insert_override_remove() {
        set_env();
        let mut set = Set::default();
        let mut rng = rand_xorshift::XorShiftRng::seed_from_u64(2);
        let mut keys = vec![];
        for _ in 0..10_000 {
            let key = rng.gen::<u64>();
            keys.push(key);
            set.insert(&key);
        }
        keys.shuffle(&mut rng);
        for key in &keys {
            assert!(set.insert(key));
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
        for _ in 0..10_000 {
            let key = rng.gen::<u64>() % 20_000;
            set_tmp.insert(key);
            set.insert(&key);
        }
        for _ in 0..10_000 {
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
        for _ in 0..10_000 {
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
        for _ in 0..100 {
            for _ in 0..=(rng.gen::<u64>() % 200 + 1) {
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
        for _ in 0..10_000 {
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
            for _ in 0..=(rng.gen::<u64>() % 200 + 1) {
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
