use crate::env;
use super::EnvStorageKey;

use std::{
    sync::Mutex,
    collections::{HashMap, HashSet},
};

#[derive(Default)]
pub struct EnvStorageCache {
    pub prefix: EnvStorageKey,
    cache: Mutex<HashMap<EnvStorageKey, Vec<u8>>>,
    dirty_keys: Mutex<HashSet<EnvStorageKey>>
}

impl EnvStorageCache {
    pub fn new(prefix: EnvStorageKey) -> Self {
        Self {
            prefix,
            cache: Mutex::new(HashMap::new()),
            dirty_keys: Mutex::new(HashSet::new())
        }
    }

    pub fn read(&self, key: &EnvStorageKey) -> Option<Vec<u8>> {
        let mut cache = self.cache.lock().expect("lock is not poisoned");
        if !cache.contains_key(key) {
            if let Some(data) = self.get_node_raw(key) {
                cache.insert(key.clone(), data);
            }
        }
        cache.get(key).map(|v| v.clone())
    }

    pub fn update(&self, key: &EnvStorageKey, value: &Vec<u8>) {
        let mut cache = self.cache.lock().expect("lock is not poisoned");
        cache.insert(key.clone(), value.clone());
        let mut dirty_keys = self.dirty_keys.lock().expect("lock is not poisoned");
        dirty_keys.insert(key.clone());
    }

    pub fn insert(&self, key: &EnvStorageKey, value: &Vec<u8>) {
        let mut cache = self.cache.lock().expect("lock is not poisoned");
        cache.insert(key.clone(), value.clone());
        self.insert_node_raw(key, value);
    }

    pub fn delete(&self, key: &EnvStorageKey) {
        self.delete_node_raw(key)
    }

    pub fn clear(&self) {
        let mut cache = self.cache.lock().expect("lock is not poisoned");
        for key in self.dirty_keys.lock().expect("lock is not poisoned").drain() {
            let node = cache.get(&key).expect("value must exist");
            self.update_node_raw(&key, node);
        }
        cache.drain();
    }

    fn get_node_raw(&self, key: &EnvStorageKey) -> Option<Vec<u8>> {
        let lookup_key = [&self.prefix, key.as_slice()].concat();
        // println!("RAW: looking up node for key {:?}", key);
        env::storage_read(&lookup_key)
    }

    fn insert_node_raw(&self, key: &EnvStorageKey, node: &Vec<u8>) {
        let lookup_key = [&self.prefix, key.as_slice()].concat();
        // println!("RAW: inserting node for key {:?}", key);
        if env::storage_write(&lookup_key, node) {
            panic!("insert node raw panic");
            // env::panic(ERR_INCONSISTENT_STATE) // Node should not exist already
        }
    }

    fn update_node_raw(&self, key: &EnvStorageKey, node: &Vec<u8>) {
        let lookup_key = [&self.prefix, key.as_slice()].concat();
        // println!("RAW: updating node for key {:?}", key);
        if !env::storage_write(&lookup_key, node) { 
            panic!("update node raw panic");
            // env::panic(ERR_INCONSISTENT_STATE) // Node should already exist
        }
    }

    fn delete_node_raw(&self, key: &EnvStorageKey) {
        let lookup_key = [&self.prefix, key.as_slice()].concat();
        if !env::storage_remove(&lookup_key) { 
            panic!("delete node raw panic. node key={:?}", key);
            // env::panic(ERR_INCONSISTENT_STATE) // Node should already exist
        }
    }
}