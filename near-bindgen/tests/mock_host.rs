//! Mock of the host running Wasm smart contract.

pub struct KV(Vec<u8>, Vec<u8>);

impl PartialEq for KV {
    fn eq(&self, other: &KV) -> bool {
        self.0.eq(&other.0)
    }
}

use primitives::types::PromiseId;
use std::collections::HashMap;
use std::mem::replace;
use wasm::ext::{Error, External};

#[derive(Default)]
pub struct KVExternal {
    // We use vector instead of `BTreeMap` for internal representation of the trie, because of how
    // we iterate over it, we cannot use regular iterators and `BTreeMap` does not provide accessing
    // elements by index.
    data: Vec<KV>,
    next_iterator_id: u32,
    iterators: HashMap<u32, (u32, Option<u32>)>,
}

impl KVExternal {
    fn entry(&self, key: &[u8]) -> Result<usize, usize> {
        self.data.binary_search_by(|el| el.0.as_slice().cmp(key))
    }

    fn entry_unchecked(&self, key: &[u8]) -> usize {
        match self.entry(key) {
            Ok(ind) => ind,
            Err(ind) => ind,
        }
    }
}

impl External for KVExternal {
    fn storage_set(&mut self, key: &[u8], value: &[u8]) -> Result<Option<Vec<u8>>, Error> {
        let kv = KV(key.to_vec(), value.to_vec());
        match self.entry(key) {
            Ok(ind) => Ok(Some(replace(&mut self.data[ind], kv).1)),
            Err(ind) => {
                self.data.insert(ind, kv);
                Ok(None)
            }
        }
    }

    fn storage_get(&self, key: &[u8]) -> Result<Option<Vec<u8>>, Error> {
        Ok(self.entry(key).map(|ind| self.data[ind].1.clone()).ok())
    }

    fn storage_remove(&mut self, key: &[u8]) -> Result<Option<Vec<u8>>, Error> {
        Ok(self.entry(key).map(|ind| self.data.remove(ind).1).ok())
    }

    fn storage_iter(&mut self, prefix: &[u8]) -> Result<u32, Error> {
        let from = self.entry_unchecked(prefix);
        self.iterators.insert(self.next_iterator_id, (from as _, None));
        self.next_iterator_id += 1;
        Ok(self.next_iterator_id - 1)
    }

    fn storage_range(&mut self, start: &[u8], end: &[u8]) -> Result<u32, Error> {
        let from = self.entry_unchecked(start);
        let to = Some(self.entry_unchecked(end) as u32);
        self.iterators.insert(self.next_iterator_id, (from as _, to));
        self.next_iterator_id += 1;
        Ok(self.next_iterator_id - 1)
    }

    /// Advances iterator. Returns `Some` if iteration is not finished yet.
    fn storage_iter_next(&mut self, id: u32) -> Result<Option<Vec<u8>>, Error> {
        let (curr, end) = self.iterators[&id];
        if end == Some(curr + 1) {
            self.iterators.remove(&id);
            return Ok(None);
        }
        match self.data.get(curr as usize) {
            Some(el) => {
                self.iterators.insert(id, (curr + 1, end));
                Ok(Some(el.1.clone()))
            }
            None => {
                self.iterators.remove(&id);
                Ok(None)
            }
        }
    }

    fn storage_iter_peek(&mut self, id: u32) -> Result<Option<Vec<u8>>, Error> {
        let (curr, _) = self.iterators[&id];
        Ok(self.data.get(curr as usize).map(|el| el.1.clone()))
    }

    fn storage_iter_remove(&mut self, id: u32) {
        self.iterators.remove(&id);
    }

    fn promise_create(
        &mut self,
        _account_id: String,
        _method_name: Vec<u8>,
        _arguments: Vec<u8>,
        _amount: u64,
    ) -> Result<PromiseId, Error> {
        unimplemented!()
    }

    fn promise_then(
        &mut self,
        _promise_id: PromiseId,
        _method_name: Vec<u8>,
        _arguments: Vec<u8>,
        _amount: u64,
    ) -> Result<PromiseId, Error> {
        unimplemented!()
    }

    fn check_ethash(
        &mut self,
        _block_number: u64,
        _header_hash: &[u8],
        _nonce: u64,
        _mix_hash: &[u8],
        _difficulty: u64,
    ) -> bool {
        unimplemented!()
    }
}
