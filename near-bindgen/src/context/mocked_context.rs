use crate::context::{AccountId, Balance, BlockIndex, Context, ResultIndex, StorageUsage};
use rand_core::RngCore;
use std::cell::RefCell;
use std::cmp::min;
use std::collections::HashMap;
use std::mem::replace;

struct KV(Vec<u8>, Vec<u8>);

impl PartialEq for KV {
    fn eq(&self, other: &KV) -> bool {
        self.0.eq(&other.0)
    }
}

/// Mocked version of `Context`.
pub struct MockedContext {
    // We use vector instead of `BTreeMap` for internal representation of the trie, because of how
    // we iterate over it, we cannot use regular iterators and `BTreeMap` does not provide accessing
    // elements by index.
    data: RefCell<Vec<KV>>,
    next_iterator_id: RefCell<u32>,
    iterators: RefCell<HashMap<u32, (u32, Option<u32>)>>,
    results: RefCell<HashMap<ResultIndex, Vec<u8>>>,
    logs: RefCell<Vec<String>>,
    originator_id: AccountId,
    account_id: AccountId,
    frozen_balance: RefCell<Balance>,
    liquid_balance: RefCell<Balance>,
    received_amount: Balance,
    storage_usage: StorageUsage,
    block_index: BlockIndex,
}

impl MockedContext {
    pub fn new() -> Self {
        Self { ..Default::default() }
    }
    fn entry(&self, key: &[u8]) -> Result<usize, usize> {
        self.data.borrow().binary_search_by(|el| el.0.as_slice().cmp(key))
    }

    fn entry_unchecked(&self, key: &[u8]) -> usize {
        match self.entry(key) {
            Ok(ind) => ind,
            Err(ind) => ind,
        }
    }
}

impl Context for MockedContext {
    fn storage_write(&self, key: &[u8], value: &[u8]) {
        let kv = KV(key.to_vec(), value.to_vec());
        match self.entry(key) {
            Ok(ind) => {
                *self.data.borrow_mut().get_mut(ind).unwrap() = kv;
            }
            Err(ind) => {
                self.data.borrow_mut().insert(ind, kv);
            }
        }
    }

    fn storage_iter(&self, prefix: &[u8]) -> u32 {
        let from = self.entry_unchecked(prefix);
        self.iterators.insert(self.next_iterator_id.borrow(), (from as _, None));
        *self.next_iterator_id.borrow_mut() += 1;
        self.next_iterator_id.borrow() - 1
    }

    fn storage_range(&self, start: &[u8], end: &[u8]) -> u32 {
        let from = self.entry_unchecked(start);
        let to = Some(self.entry_unchecked(end) as u32);
        self.iterators.insert(self.next_iterator_id.borrow(), (from as _, to));
        *self.next_iterator_id.borrow_mut() += 1;
        self.next_iterator_id.borrow() - 1
    }

    fn storage_iter_next(&self, iter_id: u32) -> bool {
        let (curr, end) = self.iterators.borrow()[&iter_id];
        if end.is_some() && end.unwrap() <= curr + 1 {
            self.iterators.remove(&iter_id);
            return false;
        }
        match self.data.get(curr as usize) {
            Some(el) => {
                self.iterators.insert(iter_id, (curr + 1, end));
                true
            }
            None => {
                self.iterators.remove(&iter_id);
                false
            }
        }
    }

    fn storage_remove(&self, key: &[u8]) {
        self.entry(key).map(|ind| self.data.borrow_mut().remove(ind).1).unwrap();
    }

    fn storage_has_key(&self, key: &[u8]) -> bool {
        self.entry(key).map(|ind| self.data.borrow()[ind].1.clone()).is_ok()
    }

    fn result_count(&self) -> u32 {
        self.results.borrow().len() as _
    }

    fn result_is_ok(&self, index: u32) -> bool {
        self.results.borrow().contains_key(&index)
    }

    fn return_value(&self, value: &[u8]) {
        unimplemented!()
    }

    fn return_promise(&self, promise_index: u32) {
        unimplemented!()
    }

    /// This method should not be called by the mocked functions.
    fn data_read(
        &self,
        data_type_index: u32,
        key_len: usize,
        key: u32,
        max_buf_len: usize,
        buf_ptr: *mut u8,
    ) -> usize {
        unimplemented!()
    }

    fn promise_create(
        &self,
        account_id: &[u8],
        method_name: &[u8],
        arguments: &[u8],
        amount: u64,
    ) -> u32 {
        unimplemented!()
    }

    fn promise_then(
        &self,
        promise_index: u32,
        method_name: &[u8],
        arguments: &[u8],
        amount: u64,
    ) -> u32 {
        unimplemented!()
    }

    fn promise_and(&self, promise_index1: u32, promise_index2: u32) -> u32 {
        unimplemented!()
    }

    fn frozen_balance(&self) -> u64 {
        *self.frozen_balance.borrow()
    }

    fn liquid_balance(&self) -> u64 {
        *self.liquid_balance.borrow()
    }

    fn deposit(&self, min_amount: u64, max_amount: u64) -> u64 {
        if *self.liquid_balance.borrow() < min_amount {
            return 0;
        }
        let delta = min(*self.liquid_balance.borrow(), max_amount);
        self.liquid_balance.borrow_mut() -= delta;
        self.frozen_balance.borrow_mut() += delta;
        delta
    }

    fn withdraw(&self, min_amount: u64, max_amount: u64) -> u64 {
        if *self.frozen_balance.borrow() < min_amount {
            return 0;
        }
        let delta = min(*self.frozen_balance.borrow(), max_amount);
        self.frozen_balance.borrow_mut() -= delta;
        self.liquid_balance.borrow_mut() += delta;
        delta
    }

    fn received_amount(&self) -> u64 {
        self.received_amount
    }

    fn storage_usage(&self) -> u64 {
        self.storage_usage
    }

    fn assert(&self, expr: bool) {
        self.assert(expr)
    }

    fn random_buf(&self, buf: &mut [u8]) {
        unimplemented!()
    }

    fn random32(&self) -> u32 {
        unimplemented!()
    }

    fn block_index(&self) -> BlockIndex {
        self.block_index
    }

    fn debug(&self, msg: &[u8]) {
        let msg = String::from_utf8(msg.to_vec()).unwrap();
        self.logs.borrow_mut().push(msg);
    }

    fn storage_read(&self, key: &[u8]) -> Vec<u8> {
        self.entry(key).map(|ind| self.data.borrow()[ind].1.clone()).unwrap_or_default()
    }

    fn storage_peek(&self, iter_id: u32) -> Vec<u8> {
        let (curr, end) = self.iterators.borrow()[&iter_id];
        if end.is_some() && curr == end.unwrap() {
            return vec![];
        }
        self.data.borrow().get(curr as usize).map(|el| el.0.clone()).unwrap_or_default()
    }

    fn originator_id(&self) -> Vec<u8> {
        self.originator_id.clone()
    }

    fn account_id(&self) -> Vec<u8> {
        self.account_id.clone()
    }
}
