pub mod unordered_set;
pub use unordered_set::*;

pub mod red_black_tree;
pub use red_black_tree::*;

pub trait Set<T> {
    /// Returns true if the set contains an element.
    fn contains(&self, element: &T) -> bool;

    /// Removes a value from the set. Returns whether the value was present in the set.
    fn remove(&mut self, element: &T) -> bool;

    /// Adds a value to the set.
    /// If the set did not have this value present, `true` is returned.
    /// If the set did have this value present, `false` is returned.
    fn insert(&mut self, element: &T) -> bool;

    /// Clears the map, removing all elements.
    fn clear(&mut self);

    /// Copies elements into an `std::vec::Vec`.
    fn to_vec(&self) -> std::vec::Vec<T>;

    /// Iterate over deserialized elements.
    fn iter<'a>(&'a self) -> Box<dyn Iterator<Item = T> + 'a>;

    fn extend<IT: IntoIterator<Item = T>>(&mut self, iter: IT);
}

#[cfg(not(target_arch = "wasm32"))]
#[cfg(test)]
pub mod tests {
    use super::Set;
    use rand::seq::SliceRandom;
    use rand::{Rng, SeedableRng};
    use std::collections::HashSet;
    use std::iter::FromIterator;

    pub fn test_insert<S: Set<u64> + Default>() {
        let mut set = S::default();
        let mut rng = rand_xorshift::XorShiftRng::seed_from_u64(0);
        for _ in 0..500 {
            let key = rng.gen::<u64>();
            set.insert(&key);
        }
    }

    pub fn test_insert_remove<S: Set<u64> + Default>() {
        let mut set = S::default();
        let mut rng = rand_xorshift::XorShiftRng::seed_from_u64(1);
        let mut keys = vec![];
        for _ in 0..250 {
            let key = rng.gen::<u64>();
            keys.push(key);
            set.insert(&key);
        }
        keys.shuffle(&mut rng);
        for key in keys {
            assert!(set.remove(&key));
        }
    }

    pub fn test_remove_last_reinsert<S: Set<u64> + Default>() {
        let mut set = S::default();
        let key1 = 1u64;
        set.insert(&key1);
        let key2 = 2u64;
        set.insert(&key2);

        let actual = set.remove(&key2);
        assert!(actual);

        let actual_reinsert = set.insert(&key2);
        assert!(actual_reinsert);
    }

    pub fn test_insert_override_remove<S: Set<u64> + Default>() {
        let mut set = S::default();
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

    pub fn test_contains_non_existent<S: Set<u64> + Default>() {
        let mut set = S::default();
        let mut rng = rand_xorshift::XorShiftRng::seed_from_u64(3);
        let mut set_tmp = HashSet::new();
        for _ in 0..250 {
            let key = rng.gen::<u64>() % 20_000;
            set_tmp.insert(key);
            set.insert(&key);
        }
        for _ in 0..250 {
            let key = rng.gen::<u64>() % 20_000;
            assert_eq!(set.contains(&key), set_tmp.contains(&key));
        }
    }

    pub fn test_to_vec<S: Set<u64> + Default>() {
        let mut set = S::default();
        let mut rng = rand_xorshift::XorShiftRng::seed_from_u64(4);
        let mut keys = HashSet::new();
        for _ in 0..500 {
            let key = rng.gen::<u64>();
            keys.insert(key);
            set.insert(&key);
        }
        let actual = HashSet::from_iter(set.to_vec());
        assert_eq!(actual, keys);
    }

    pub fn test_clear<S: Set<u64> + Default>() {
        let mut set = S::default();
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

    pub fn test_iter<S: Set<u64> + Default>() {
        let mut set = S::default();
        let mut rng = rand_xorshift::XorShiftRng::seed_from_u64(4);
        let mut keys = HashSet::new();
        for _ in 0..500 {
            let key = rng.gen::<u64>();
            keys.insert(key);
            set.insert(&key);
        }
        let actual: HashSet<u64> = HashSet::from_iter(set.iter());
        assert_eq!(actual, keys);
    }

    pub fn test_extend<S: Set<u64> + Default>() {
        let mut set = S::default();
        let mut rng = rand_xorshift::XorShiftRng::seed_from_u64(4);
        let mut keys = HashSet::new();
        for _ in 0..100 {
            let key = rng.gen::<u64>();
            keys.insert(key);
            set.insert(&key);
        }
        for _ in 0..10 {
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