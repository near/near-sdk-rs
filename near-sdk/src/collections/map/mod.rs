pub mod unordered_map;
pub use unordered_map::*;

pub mod ordered_map;
pub use ordered_map::*;

pub trait Map<K, V> {
    /// Returns the value corresponding to the key.
    fn get(&self, key: &K) -> Option<V>;

    /// Removes a key from the map, returning the value at the key if the key was previously in the
    /// map.
    fn remove(&mut self, key: &K) -> Option<V>;

    /// Inserts a key-value pair into the map.
    /// If the map did not have this key present, `None` is returned. Otherwise returns
    /// a value. 
    fn insert(&mut self, key: &K, value: &V) -> Option<V>;

    /// Clears the map, removing all elements.
    fn clear(&mut self);

    /// Copies elements into an `std::vec::Vec`.
    fn to_vec(&self) -> std::vec::Vec<(K, V)>;

    /// An iterator visiting all keys. The iterator element type is `K`.
    fn keys<'a>(&'a self) -> Box<dyn Iterator<Item = K> + 'a>;

    /// An iterator visiting all values. The iterator element type is `V`.
    fn values<'a>(&'a self) -> Box<dyn Iterator<Item = V> + 'a>;

    /// Iterate over deserialized keys and values.
    fn iter<'a>(&'a self) -> Box<dyn Iterator<Item = (K, V)> + 'a>;

    fn extend<IT: IntoIterator<Item = (K, V)>>(&mut self, iter: IT) where Self: Sized;
}

#[cfg(not(target_arch = "wasm32"))]
#[cfg(test)]
pub mod tests {
    use super::Map;
    use rand::seq::SliceRandom;
    use rand::{Rng, SeedableRng};
    use std::collections::{HashMap, HashSet};
    use std::iter::FromIterator;


    pub fn test_insert<M: Map<u64, u64> + Default>() {
        let mut map = M::default();
        let mut rng = rand_xorshift::XorShiftRng::seed_from_u64(0);
        for _ in 0..100 {
            let key = rng.gen::<u64>();
            let value = rng.gen::<u64>();
            map.insert(&key, &value);
        }
    }


    pub fn test_insert_remove<M: Map<u64, u64> + Default>() {
        let mut map = M::default();
        let mut rng = rand_xorshift::XorShiftRng::seed_from_u64(1);
        let mut keys = vec![];
        let mut key_to_value = HashMap::new();
        for _ in 0..100 {
            let key = rng.gen::<u64>();
            let value = rng.gen::<u64>();
            keys.push(key);
            key_to_value.insert(key, value);
            map.insert(&key, &value);
        }
        keys.shuffle(&mut rng);
        for key in keys {
            let actual = map.remove(&key).unwrap();
            assert_eq!(actual, key_to_value[&key]);
        }
    }


    pub fn test_remove_last_reinsert<M: Map<u64, u64> + Default>() {
        let mut map = M::default();
        let key1 = 1u64;
        let value1 = 2u64;
        map.insert(&key1, &value1);
        let key2 = 3u64;
        let value2 = 4u64;
        map.insert(&key2, &value2);

        let actual_value2 = map.remove(&key2).unwrap();
        assert_eq!(actual_value2, value2);

        let actual_insert_value2 = map.insert(&key2, &value2);
        assert_eq!(actual_insert_value2, None);
    }


    pub fn test_insert_override_remove<M: Map<u64, u64> + Default>() {
        let mut map = M::default();
        let mut rng = rand_xorshift::XorShiftRng::seed_from_u64(2);
        let mut keys = vec![];
        let mut key_to_value = HashMap::new();
        for _ in 0..100 {
            let key = rng.gen::<u64>();
            let value = rng.gen::<u64>();
            keys.push(key);
            key_to_value.insert(key, value);
            map.insert(&key, &value);
        }
        keys.shuffle(&mut rng);
        for key in &keys {
            let value = rng.gen::<u64>();
            let actual = map.insert(key, &value).unwrap();
            assert_eq!(actual, key_to_value[key]);
            key_to_value.insert(*key, value);
        }
        keys.shuffle(&mut rng);
        for key in keys {
            let actual = map.remove(&key).unwrap();
            assert_eq!(actual, key_to_value[&key]);
        }
    }


    pub fn test_get_non_existent<M: Map<u64, u64> + Default>() {
        let mut map = M::default();
        let mut rng = rand_xorshift::XorShiftRng::seed_from_u64(3);
        let mut key_to_value = HashMap::new();
        for _ in 0..250 {
            let key = rng.gen::<u64>() % 20_000;
            let value = rng.gen::<u64>();
            key_to_value.insert(key, value);
            map.insert(&key, &value);
        }
        for _ in 0..250 {
            let key = rng.gen::<u64>() % 20_000;
            assert_eq!(map.get(&key), key_to_value.get(&key).cloned());
        }
    }


    pub fn test_to_vec<M: Map<u64, u64> + Default>() {
        let mut map = M::default();
        let mut rng = rand_xorshift::XorShiftRng::seed_from_u64(4);
        let mut key_to_value = HashMap::new();
        for _ in 0..250 {
            let key = rng.gen::<u64>();
            let value = rng.gen::<u64>();
            key_to_value.insert(key, value);
            map.insert(&key, &value);
        }
        let actual = HashMap::from_iter(map.to_vec());
        assert_eq!(actual, key_to_value);
    }


    pub fn test_clear<M: Map<u64, u64> + Default>() {
        let mut map = M::default();
        let mut rng = rand_xorshift::XorShiftRng::seed_from_u64(5);
        for _ in 0..10 {
            for _ in 0..=(rng.gen::<u64>() % 20 + 1) {
                let key = rng.gen::<u64>();
                let value = rng.gen::<u64>();
                map.insert(&key, &value);
            }
            assert!(!map.to_vec().is_empty());
            map.clear();
            assert!(map.to_vec().is_empty());
        }
    }


    pub fn test_keys_values<M: Map<u64, u64> + Default>() {
        let mut map = M::default();
        let mut rng = rand_xorshift::XorShiftRng::seed_from_u64(4);
        let mut key_to_value = HashMap::new();
        for _ in 0..250 {
            let key = rng.gen::<u64>();
            let value = rng.gen::<u64>();
            key_to_value.insert(key, value);
            map.insert(&key, &value);
        }
        let actual: HashMap<u64, u64> = HashMap::from_iter(map.to_vec());
        assert_eq!(
            actual.keys().collect::<HashSet<_>>(),
            key_to_value.keys().collect::<HashSet<_>>()
        );
        assert_eq!(
            actual.values().collect::<HashSet<_>>(),
            key_to_value.values().collect::<HashSet<_>>()
        );
    }


    pub fn test_iter<M: Map<u64, u64> + Default>() {
        let mut map = M::default();
        let mut rng = rand_xorshift::XorShiftRng::seed_from_u64(4);
        let mut key_to_value = HashMap::new();
        for _ in 0..250 {
            let key = rng.gen::<u64>();
            let value = rng.gen::<u64>();
            key_to_value.insert(key, value);
            map.insert(&key, &value);
        }
        let actual: HashMap<u64, u64> = HashMap::from_iter(map.iter());
        assert_eq!(actual, key_to_value);
    }


    pub fn test_extend<M: Map<u64, u64> + Default>() {
        let mut map = M::default();
        let mut rng = rand_xorshift::XorShiftRng::seed_from_u64(4);
        let mut key_to_value = HashMap::new();
        for _ in 0..100 {
            let key = rng.gen::<u64>();
            let value = rng.gen::<u64>();
            key_to_value.insert(key, value);
            map.insert(&key, &value);
        }
        for _ in 0..10 {
            let mut tmp = vec![];
            for _ in 0..=(rng.gen::<u64>() % 20 + 1) {
                let key = rng.gen::<u64>();
                let value = rng.gen::<u64>();
                tmp.push((key, value));
            }
            key_to_value.extend(tmp.iter().cloned());
            map.extend(tmp.iter().cloned());
        }

        let actual: HashMap<u64, u64> = HashMap::from_iter(map.iter());
        assert_eq!(actual, key_to_value);
    }
}