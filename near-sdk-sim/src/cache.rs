use crate::types::CompiledContractCache;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;

pub struct ContractCache {
    data: Rc<RefCell<HashMap<Vec<u8>, Vec<u8>>>>,
}

impl Clone for ContractCache {
    fn clone(&self) -> Self {
        Self { data: Rc::clone(&self.data) }
    }

    fn clone_from(&mut self, source: &Self) {
        self.data = Rc::clone(&source.data)
    }
}

impl ContractCache {
    pub fn new() -> Self {
        Self { data: Rc::new(RefCell::new(HashMap::new())) }
    }

    pub fn insert(&self, key: &[u8], value: &[u8]) -> Option<Vec<u8>> {
        (*self.data).borrow_mut().insert((*key).to_owned(), (*value).to_owned())
    }

    pub fn get(&self, key: &[u8]) -> Option<Vec<u8>> {
        match (*self.data).borrow_mut().get(key) {
            Some(v) => Some(v.clone()),
            _ => None,
        }
    }

    pub fn to_arc(&self) -> Arc<ContractCache> {
        Arc::new(self.clone())
    }
}

impl CompiledContractCache for ContractCache {
    fn put(&self, key: &[u8], value: &[u8]) -> Result<(), std::io::Error> {
        self.insert(key, value);
        Ok(())
    }

    fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>, std::io::Error> {
        Ok(self.get(key))
    }
}

unsafe impl Send for ContractCache {}
unsafe impl Sync for ContractCache {}
