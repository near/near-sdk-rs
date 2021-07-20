use crate::types::CompiledContractCache;
use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

/// This provides a disc cache for compiled contracts.
/// The cached contracts are located `CARGO_MANIFEST_DIR/target/contract_cache`.
#[derive(Clone, Default)]
pub struct ContractCache {
    data: Arc<Mutex<HashMap<Vec<u8>, Vec<u8>>>>,
}

pub(crate) fn key_to_b58(key: &[u8]) -> String {
    near_sdk::bs58::encode(key).into_string()
}

impl ContractCache {
    pub fn new() -> Self {
        ContractCache::default()
    }

    fn path() -> PathBuf {
        let s = std::env::var("CARGO_MANIFEST_DIR").unwrap();
        Path::new(&s).join("target").join("contract_cache")
    }

    fn open_file(&self, key: &[u8]) -> std::io::Result<File> {
        let path = self.get_path(key);
        // Ensure that the parent path exists
        let prefix = path.parent().unwrap();
        std::fs::create_dir_all(prefix).unwrap();
        // Ensure we can read, write, and create file if it doesn't exist
        OpenOptions::new().read(true).write(true).create(true).open(path)
    }

    fn get_path(&self, key: &[u8]) -> PathBuf {
        ContractCache::path().join(key_to_b58(key))
    }

    fn file_exists(&self, key: &[u8]) -> bool {
        self.get_path(key).exists()
    }

    pub fn insert(&self, key: &[u8], value: &[u8]) -> Option<Vec<u8>> {
        self.data.lock().unwrap().insert(key.to_vec(), value.to_vec())
    }

    pub fn get(&self, key: &[u8]) -> Option<Vec<u8>> {
        self.data.lock().unwrap().get(key).cloned()
    }

    #[allow(dead_code)]
    pub(crate) fn to_arc(&self) -> Arc<ContractCache> {
        Arc::new(self.clone())
    }
}

impl CompiledContractCache for ContractCache {
    fn put(&self, key: &[u8], value: &[u8]) -> Result<(), std::io::Error> {
        self.insert(key, value);
        let mut file = self.open_file(key).expect("File failed to open");
        let metadata = file.metadata()?;
        if metadata.len() != value.len() as u64 {
            file.write_all(value)?;
        }
        Ok(())
    }

    fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>, std::io::Error> {
        if (*self.data).lock().unwrap().contains_key(key) {
            return Ok(self.get(key));
        } else if self.file_exists(key) {
            let mut file = self.open_file(key)?;
            let mut contents = vec![];
            file.read_to_end(&mut contents)?;
            self.insert(key, &contents);
            return Ok(Some(contents));
        }
        Ok(None)
    }
}

pub fn create_cache() -> ContractCache {
    ContractCache::new()
}

pub fn cache_to_arc(cache: &ContractCache) -> Arc<ContractCache> {
    cache.to_arc()
}
