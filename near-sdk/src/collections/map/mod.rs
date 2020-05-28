pub mod unordered_map;
pub use unordered_map::*;

pub mod ordered_map;
pub use ordered_map::*;

use std::ops::Bound;

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

// Adapted from TreeMap implementation by https://github.com/sergey-melnychuk 
// https://github.com/near/near-sdk-rs/blob/45035d5909e200a1ba03b779435de9d08b7e7f4c/near-sdk/src/collections/tree_map.rs
pub trait TreeMap<K, V>: Map<K, V> {
    /// Returns true if the tree contains the key, false otherwise
    fn contains_key(&self, key: &K) -> bool;
    
    /// Returns the smallest stored key from the tree
    fn min(&self) -> Option<V>;
    
    /// Returns the largest stored key from the tree
    fn max(&self) -> Option<V>;

    /// Returns the smallest key that is strictly greater than key given as the parameter
    fn above(&self, key: &K) -> Option<K>;

    /// Returns the largest key that is strictly less than key given as the parameter
    fn below(&self, key: &K) -> Option<K>;

    /// Returns the largest key that is greater or equal to key given as the parameter
    fn ceil(&self) -> Option<V>;
    
    /// Returns the smallest key that is greater or equal to key given as the parameter
    fn floor(&self) -> Option<V>;

    /// Iterates through keys in ascending order starting at key that is greater than
    /// or equal to the key supplied
    fn iter_from<'a>(&'a self, key: &K) -> Box<dyn Iterator<Item = (K, V)> + 'a>;

    /// Iterates through keys in descending order
    fn iter_rev<'a>(&'a self) -> Box<dyn Iterator<Item = (K, V)> + 'a>;

    /// Iterates through keys in descending order starting at key that is less than
    /// or equal to the key supplied
    fn iter_rev_from<'a>(&'a self, key: &K) -> Box<dyn Iterator<Item = (K, V)> + 'a>;

    /// Iterate over K keys in ascending order
    ///
    /// # Panics
    ///
    /// Panics if range start > end.
    /// Panics if range start == end and both bounds are Excluded.
    fn range<'a>(&'a self, r: (Bound<K>, Bound<K>)) -> Box<dyn Iterator<Item = (K, V)> + 'a>;
}

#[cfg(not(target_arch = "wasm32"))]
#[cfg(test)]
pub mod tests {
    use super::{Map, TreeMap};
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

    /*
    // Tests from https://github.com/near/near-sdk-rs/blob/bb94ec0943b7adb5d1c97845ad0e103314f7cd5c/near-sdk/src/collections/tree_map.rs
    // credit to https://github.com/sergey-melnychuk
    fn test_empty<M: TreeMap<u8, u8> + Default>() {

        let map = M::default();
        assert_eq!(map.len(), 0);
        // assert_eq!(map.height(), 0);
        assert_eq!(map.get(&42), None);
        assert!(!map.contains_key(&42));
        assert_eq!(map.min(), None);
        assert_eq!(map.max(), None);
        assert_eq!(map.ceil(&42), None);
        assert_eq!(map.floor(&42), None);
    }

    fn test_insert_3_rotate_l_l() {

        let mut map: TreeMap<i32, i32> = TreeMap::default();
        // assert_eq!(map.height(), 0);

        map.insert(&3, &3);
        // assert_eq!(map.height(), 1);

        map.insert(&2, &2);
        // assert_eq!(map.height(), 2);

        map.insert(&1, &1);

        let root = map.root;
        assert_eq!(root, 1);
        assert_eq!(map.key.get(&root), Some(2));
        // assert_eq!(map.height(), 2);

        map.clear();
    }

    fn test_insert_3_rotate_r_r() {

        let mut map: TreeMap<i32, i32> = TreeMap::default();
        // assert_eq!(map.height(), 0);

        map.insert(&1, &1);
        // assert_eq!(map.height(), 1);

        map.insert(&2, &2);
        // assert_eq!(map.height(), 2);

        map.insert(&3, &3);

        let root = map.root;
        assert_eq!(root, 1);
        assert_eq!(map.key.get(&root), Some(2));
        // assert_eq!(map.height(), 2);

        map.clear();
    }

    fn test_insert_lookup_n_asc() {

        let mut map: TreeMap<i32, i32> = TreeMap::default();

        let n: u64 = 30;
        let cases = (0..2*(n as i32)).collect::<Vec<i32>>();

        let mut counter  = 0;
        for k in &cases {
            if *k % 2 == 0 {
                counter += 1;
                map.insert(k, &counter);
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

        // assert!(map.height() <= max_tree_height(n));
        map.clear();
    }

    fn test_insert_lookup_n_desc() {

        let mut map: TreeMap<i32, i32> = TreeMap::default();

        let n: u64 = 30;
        let cases = (0..2*(n as i32)).rev().collect::<Vec<i32>>();

        let mut counter  = 0;
        for k in &cases {
            if *k % 2 == 0 {
                counter += 1;
                map.insert(k, &counter);
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

        // assert!(map.height() <= max_tree_height(n));
        map.clear();
    }

    fn insert_n_random() {
        test_env::setup_free();

        for k in 1..10 { // tree size is 2^k
            let mut map: TreeMap<u32, u32> = TreeMap::default();

            let n = 1 << k;
            let input: Vec<u32> = random(n);

            for x in &input {
                map.insert(x, &42);
            }

            // assert!(map.height() <= max_tree_height(n));
            map.clear();
        }
    }

    fn test_min() {

        let n: u64 = 30;
        let vec = random(n);

        let mut map: TreeMap<u32, u32> = TreeMap::new(vec![b't']);
        for x in vec.iter().rev() {
            map.insert(x, &1);
        }

        assert_eq!(map.min().unwrap(), *vec.iter().min().unwrap());
        map.clear();
    }

    fn test_max() {

        let n: u64 = 30;
        let vec = random(n);

        let mut map: TreeMap<u32, u32> = TreeMap::new(vec![b't']);
        for x in vec.iter().rev() {
            map.insert(x, &1);
        }

        assert_eq!(map.max().unwrap(), *vec.iter().max().unwrap());
        map.clear();
    }

    fn test_ceil() {

        let mut map: TreeMap<u32, u32> = TreeMap::default();
        let vec: Vec<u32> = vec![10, 20, 30, 40, 50];

        for x in vec.iter() {
            map.insert(x, &1);
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

    fn test_floor() {

        let mut map: TreeMap<u32, u32> = TreeMap::default();
        let vec: Vec<u32> = vec![10, 20, 30, 40, 50];

        for x in vec.iter() {
            map.insert(x, &1);
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

    fn test_remove_1() {

        let mut map: TreeMap<u32, u32> = TreeMap::default();
        map.insert(&1, &1);
        assert_eq!(map.get(&1), Some(1));
        map.remove(&1);
        assert_eq!(map.get(&1), None);
        assert_eq!(map.key.len(), 0);
        assert_eq!(map.ht.len(), 0);
        map.clear();
    }

    fn test_remove_3_desc() {

        let vec: Vec<u32> = vec![3, 2, 1];
        let mut map: TreeMap<u32, u32> = TreeMap::default();

        for x in &vec {
            assert_eq!(map.get(x), None);
            map.insert(x, &1);
            assert_eq!(map.get(x), Some(1));
        }

        for x in &vec {
            assert_eq!(map.get(x), Some(1));
            map.remove(x);
            assert_eq!(map.get(x), None);
        }
        map.clear();
    }

    fn test_remove_3_asc() {

        let vec: Vec<u32> = vec![1, 2, 3];
        let mut map: TreeMap<u32, u32> = TreeMap::default();

        for x in &vec {
            assert_eq!(map.get(x), None);
            map.insert(x, &1);
            assert_eq!(map.get(x), Some(1));
        }

        for x in &vec {
            assert_eq!(map.get(x), Some(1));
            map.remove(x);
            assert_eq!(map.get(x), None);
        }
        map.clear();
    }

    fn test_remove_7_regression_1() {

        let vec: Vec<u32> = vec![2104297040, 552624607, 4269683389, 3382615941,
                                 155419892, 4102023417, 1795725075];
        let mut map: TreeMap<u32, u32> = TreeMap::default();

        for x in &vec {
            assert_eq!(map.get(x), None);
            map.insert(x, &1);
            assert_eq!(map.get(x), Some(1));
        }

        for x in &vec {
            assert_eq!(map.get(x), Some(1));
            map.remove(x);
            assert_eq!(map.get(x), None);
        }
        map.clear();
    }

    fn test_remove_7_regression_2() {

        let vec: Vec<u32> = vec![700623085, 87488544, 1500140781, 1111706290,
                                 3187278102, 4042663151, 3731533080];
        let mut map: TreeMap<u32, u32> = TreeMap::default();

        for x in &vec {
            assert_eq!(map.get(x), None);
            map.insert(x, &1);
            assert_eq!(map.get(x), Some(1));
        }

        for x in &vec {
            assert_eq!(map.get(x), Some(1));
            map.remove(x);
            assert_eq!(map.get(x), None);
        }
        map.clear();
    }

    fn test_remove_9_regression() {

        let vec: Vec<u32> = vec![1186903464, 506371929, 1738679820, 1883936615, 1815331350,
                                 1512669683, 3581743264, 1396738166, 1902061760];
        let mut map: TreeMap<u32, u32> = TreeMap::default();

        for x in &vec {
            assert_eq!(map.get(x), None);
            map.insert(x, &1);
            assert_eq!(map.get(x), Some(1));
        }

        for x in &vec {
            assert_eq!(map.get(x), Some(1));
            map.remove(x);
            assert_eq!(map.get(x), None);
        }
        map.clear();
    }

    fn test_remove_20_regression_1() {

        let vec: Vec<u32> = vec![552517392, 3638992158, 1015727752, 2500937532, 638716734,
                                 586360620, 2476692174, 1425948996, 3608478547, 757735878,
                                 2709959928, 2092169539, 3620770200, 783020918, 1986928932,
                                 200210441, 1972255302, 533239929, 497054557, 2137924638];
        let mut map: TreeMap<u32, u32> = TreeMap::default();

        for x in &vec {
            assert_eq!(map.get(x), None);
            map.insert(x, &1);
            assert_eq!(map.get(x), Some(1));
        }

        for x in &vec {
            assert_eq!(map.get(x), Some(1));
            map.remove(x);
            assert_eq!(map.get(x), None);
        }
        map.clear();
    }

    fn test_remove_7_regression() {

        let vec: Vec<u32> = vec![280, 606, 163, 857, 436, 508, 44, 801];

        let mut map: TreeMap<u32, u32> = TreeMap::default();

        for x in &vec {
            assert_eq!(map.get(x), None);
            map.insert(x, &1);
            assert_eq!(map.get(x), Some(1));
        }

        for x in &vec {
            assert_eq!(map.get(x), Some(1));
            map.remove(x);
            assert_eq!(map.get(x), None);
        }

        assert_eq!(map.len(), 0, "map.len() > 0");
        assert_eq!(map.key.len(), 0, "map.key is not empty");
        assert_eq!(map.val.len(), 0, "map.val is not empty");
        assert_eq!(map.ht.len(),  0, "map.ht  is not empty");
        assert_eq!(map.lft.len(), 0, "map.lft is not empty");
        assert_eq!(map.rgt.len(), 0, "map.rgt is not empty");
        map.clear();
    }

    fn test_remove_n() {

        let n: u64 = 20;
        let vec = random(n);

        let mut set: HashSet<u32> = HashSet::new();
        let mut map: TreeMap<u32, u32> = TreeMap::default();
        for x in &vec {
            map.insert(x, &1);
            set.insert(*x);
        }

        assert_eq!(map.len(), set.len() as u64);

        for x in &set {
            assert_eq!(map.get(x), Some(1));
            map.remove(x);
            assert_eq!(map.get(x), None);
        }

        assert_eq!(map.len(), 0, "map.len() > 0");

        assert_eq!(map.key.len(), 0, "map.key is not empty");
        assert_eq!(map.val.len(), 0, "map.val is not empty");
        assert_eq!(map.ht.len(),  0, "map.ht  is not empty");
        assert_eq!(map.lft.len(), 0, "map.lft is not empty");
        assert_eq!(map.rgt.len(), 0, "map.rgt is not empty");
        map.clear();
    }

    fn test_remove_root_3() {

        let mut map: TreeMap<u32, u32> = TreeMap::default();
        map.insert(&2, &1);
        map.insert(&3, &1);
        map.insert(&1, &1);
        map.insert(&4, &1);

        map.remove(&2);

        assert_eq!(map.get(&1), Some(1));
        assert_eq!(map.get(&2), None);
        assert_eq!(map.get(&3), Some(1));
        assert_eq!(map.get(&4), Some(1));
        map.clear();
    }

    fn test_insert_2_remove_2_regression() {

        let ins: Vec<u32> = vec![11760225, 611327897];
        let rem: Vec<u32> = vec![2982517385, 1833990072];

        let mut map: TreeMap<u32, u32> = TreeMap::default();
        map.insert(&ins[0], &1);
        map.insert(&ins[1], &1);

        map.remove(&rem[0]);
        map.remove(&rem[1]);

        // let h = map.height();
        // let h_max = max_tree_height(map.len());
        // assert!(h <= h_max, "h={} h_max={}", h, h_max);
        map.clear();
    }

    fn test_insert_n_duplicates() {
        let mut map: TreeMap<u32, u32> = TreeMap::default();

        for x in 0..30 {
            map.insert(&x, &x);
            map.insert(&42, &x);
        }

        assert_eq!(map.get(&42), Some(29));
        assert_eq!(map.len(), 31);
        assert_eq!(map.key.len(), 31);
        assert_eq!(map.ht.len(), 31);

        map.clear();
    }

    fn test_insert_2n_remove_n_random() {

        for k in 1..4 {
            let mut map: TreeMap<u32, u32> = TreeMap::default();
            let mut set: HashSet<u32> = HashSet::new();

            let n = 1 << k;
            let ins: Vec<u32> = random(n);
            let rem: Vec<u32> = random(n);

            for x in &ins {
                set.insert(*x);
                map.insert(x, &42);
            }

            for x in &rem {
                set.insert(*x);
                map.insert(x, &42);
            }

            for x in &rem {
                set.remove(x);
                map.remove(x);
            }

            assert_eq!(map.len(), set.len() as u64);

            // let h = map.height();
            let h_max = max_tree_height(n);
            assert!(h <= h_max, "[n={}] tree is too high: {} (max is {}).", n, h, h_max);

            map.clear();
        }
    }

    fn test_remove_empty() {
        let mut map: TreeMap<u32, u32> = TreeMap::default();
        assert_eq!(map.remove(&1), None);
    }

    fn test_to_vec_empty() {
        let map: TreeMap<u32, u32> = TreeMap::default();
        assert!(map.to_vec().is_empty());
    }

    fn test_iter_empty() {
        let map: TreeMap<u32, u32> = TreeMap::default();
        assert!(map.iter().collect::<Vec<(u32, u32)>>().is_empty());
    }

    fn test_iter_rev() {
        let mut map: TreeMap<u32, u32> = TreeMap::default();
        map.insert(&1, &41);
        map.insert(&2, &42);
        map.insert(&3, &43);

        assert_eq!(map.iter_rev().collect::<Vec<(u32, u32)>>(), vec![(3, 43), (2, 42), (1, 41)]);
        map.clear();
    }

    fn test_iter_rev_empty() {
        let map: TreeMap<u32, u32> = TreeMap::default();
        assert!(map.iter_rev().collect::<Vec<(u32, u32)>>().is_empty());
    }

    fn test_iter_from() {
        let mut map: TreeMap<u32, u32> = TreeMap::default();

        let one: Vec<u32> = vec![10, 20, 30, 40, 50];
        let two: Vec<u32> = vec![45, 35, 25, 15, 5];

        for x in &one {
            map.insert(x, &42);
        }

        for x in &two {
            map.insert(x, &42);
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

    fn test_iter_from_empty() {
        let map: TreeMap<u32, u32> = TreeMap::default();
        assert!(map.iter_from(42).collect::<Vec<(u32, u32)>>().is_empty());
    }

    fn test_iter_rev_from() {
        let mut map: TreeMap<u32, u32> = TreeMap::default();

        let one: Vec<u32> = vec![10, 20, 30, 40, 50];
        let two: Vec<u32> = vec![45, 35, 25, 15, 5];

        for x in &one {
            map.insert(x, &42);
        }

        for x in &two {
            map.insert(x, &42);
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

    fn test_range() {
        let mut map: TreeMap<u32, u32> = TreeMap::default();

        let one: Vec<u32> = vec![10, 20, 30, 40, 50];
        let two: Vec<u32> = vec![45, 35, 25, 15, 5];

        for x in &one {
            map.insert(x, &42);
        }

        for x in &two {
            map.insert(x, &42);
        }

        assert_eq!(
            map.range((Bound::Included(20), Bound::Excluded(30))).collect::<Vec<(u32, u32)>>(),
            vec![(20, 42), (25, 42)]);

        assert_eq!(
            map.range((Bound::Excluded(10), Bound::Included(40))).collect::<Vec<(u32, u32)>>(),
            vec![(15, 42), (20, 42), (25, 42), (30, 42), (35, 42), (40, 42)]);

        assert_eq!(
            map.range((Bound::Included(20), Bound::Included(40))).collect::<Vec<(u32, u32)>>(),
            vec![(20, 42), (25, 42), (30, 42), (35, 42), (40, 42)]);

        assert_eq!(
            map.range((Bound::Excluded(20), Bound::Excluded(45))).collect::<Vec<(u32, u32)>>(),
            vec![(25, 42), (30, 42), (35, 42), (40, 42)]);

        assert_eq!(
            map.range((Bound::Excluded(20), Bound::Excluded(45))).collect::<Vec<(u32, u32)>>(),
            vec![(25, 42), (30, 42), (35, 42), (40, 42)]);

        assert_eq!(
            map.range((Bound::Excluded(25), Bound::Excluded(30))).collect::<Vec<(u32, u32)>>(),
            vec![]);

        assert_eq!(
            map.range((Bound::Included(25), Bound::Included(25))).collect::<Vec<(u32, u32)>>(),
            vec![(25, 42)]);

        assert_eq!(
            map.range((Bound::Excluded(25), Bound::Included(25))).collect::<Vec<(u32, u32)>>(),
            vec![]); // the range makes no sense, but `BTreeMap` does not panic in this case

        map.clear();
    }

    #[should_panic(expected = "Invalid range.")]
    fn test_range_panics_same_excluded() {
        let map: TreeMap<u32, u32> = TreeMap::default();
        let _ = map.range((Bound::Excluded(1), Bound::Excluded(1)));
    }

    #[should_panic(expected = "Invalid range.")]
    fn test_range_panics_non_overlap_incl_exlc() {
        let map: TreeMap<u32, u32> = TreeMap::default();
        let _ = map.range((Bound::Included(2), Bound::Excluded(1)));
    }

    #[should_panic(expected = "Invalid range.")]
    fn test_range_panics_non_overlap_excl_incl() {
        let map: TreeMap<u32, u32> = TreeMap::default();
        let _ = map.range((Bound::Excluded(2), Bound::Included(1)));
    }

    #[should_panic(expected = "Invalid range.")]
    fn test_range_panics_non_overlap_incl_incl() {
        let map: TreeMap<u32, u32> = TreeMap::default();
        let _ = map.range((Bound::Included(2), Bound::Included(1)));
    }

    fn test_iter_rev_from_empty() {
        let map: TreeMap<u32, u32> = TreeMap::default();
        assert!(map.iter_rev_from(42).collect::<Vec<(u32, u32)>>().is_empty());
    }

    //
    // Property-based tests of AVL-based TreeMap against std::collections::BTreeMap
    //

    // fn avl<K, V>(insert: &[(K, V)], remove: &[K]) -> TreeMap<K, V>
    //     where
    //         K: Ord + Copy + BorshSerialize + BorshDeserialize,
    //         V: Copy + BorshSerialize + BorshDeserialize,
    // {
    //     test_env::setup_free();
    //     let mut map: TreeMap<K, V> = TreeMap::default();
    //     for (k, v) in insert {
    //         map.insert(k, v);
    //     }
    //     for k in remove {
    //         map.remove(k);
    //     }
    //     map
    // }

    // fn rb<K, V>(insert: &[(K, V)], remove: &[K]) -> BTreeMap<K, V>
    //     where
    //         K: Ord + Copy + BorshSerialize + BorshDeserialize,
    //         V: Copy + BorshSerialize + BorshDeserialize,
    // {
    //     let mut map: BTreeMap<K, V> = BTreeMap::default();
    //     for (k, v) in insert {
    //         map.insert(*k, *v);
    //     }
    //     for k in remove {
    //         map.remove(k);
    //     }
    //     map
    // }

    // fn prop_avl_vs_rb() {
    //     fn prop(insert: Vec<(u32, u32)>, remove: Vec<u32>) -> bool {
    //         let a = avl(&insert, &remove);
    //         let b = rb(&insert, &remove);
    //         let v1: Vec<(u32, u32)> = a.iter().collect();
    //         let v2: Vec<(u32, u32)> = b.into_iter().collect();
    //         v1 == v2
    //     }

    //     QuickCheck::new()
    //         .tests(300)
    //         .quickcheck(prop as fn(std::vec::Vec<(u32, u32)>, std::vec::Vec<u32>) -> bool);
    // }

    // fn range_prop(insert: Vec<(u32, u32)>, remove: Vec<u32>, range: (Bound<u32>, Bound<u32>)) -> bool {
    //     let a = avl(&insert, &remove);
    //     let b = rb(&insert, &remove);
    //     let v1: Vec<(u32, u32)> = a.range(range).collect();
    //     let v2: Vec<(u32, u32)> = b.range(range)
    //         .map(|(k, v)| (*k, *v))
    //         .collect();
    //     v1 == v2 || {
    //         println!("\ninsert: {:?}", insert);
    //         println!("remove: {:?}", remove);
    //         println!(" range: {:?}", range);
    //         println!("AVL: {:?}", v1);
    //         println!(" RB: {:?}", v2);
    //         false
    //     }
    // }

    // type Prop = fn(std::vec::Vec<(u32, u32)>, std::vec::Vec<u32>, u32, u32) -> bool;

    // fn prop_avl_vs_rb_range_incl_incl() {
    //     fn prop(insert: Vec<(u32, u32)>, remove: Vec<u32>, r1: u32, r2: u32) -> bool {
    //         let range = (Bound::Included(r1.min(r2)), Bound::Included(r1.max(r2)));
    //         range_prop(insert, remove, range)
    //     }

    //     QuickCheck::new()
    //         .tests(300)
    //         .quickcheck(prop as Prop);
    // }

    // fn prop_avl_vs_rb_range_incl_excl() {
    //     fn prop(insert: Vec<(u32, u32)>, remove: Vec<u32>, r1: u32, r2: u32) -> bool {
    //         let range = (Bound::Included(r1.min(r2)), Bound::Excluded(r1.max(r2)));
    //         range_prop(insert, remove, range)
    //     }

    //     QuickCheck::new()
    //         .tests(300)
    //         .quickcheck(prop as Prop);
    // }

    // fn prop_avl_vs_rb_range_excl_incl() {
    //     fn prop(insert: Vec<(u32, u32)>, remove: Vec<u32>, r1: u32, r2: u32) -> bool {
    //         let range = (Bound::Excluded(r1.min(r2)), Bound::Included(r1.max(r2)));
    //         range_prop(insert, remove, range)
    //     }

    //     QuickCheck::new()
    //         .tests(300)
    //         .quickcheck(prop as Prop);
    // }

    // fn prop_avl_vs_rb_range_excl_excl() {
    //     fn prop(insert: Vec<(u32, u32)>, remove: Vec<u32>, r1: u32, r2: u32) -> bool {
    //         // (Excluded(x), Excluded(x)) is invalid range, checking against it makes no sense
    //         r1 == r2 || {
    //             let range = (Bound::Excluded(r1.min(r2)), Bound::Excluded(r1.max(r2)));
    //             range_prop(insert, remove, range)
    //         }
    //     }

    //     QuickCheck::new()
    //         .tests(300)
    //         .quickcheck(prop as Prop);
    // }
    */
}