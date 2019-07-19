use crate::context::{AccountId, Balance, BlockIndex, Context, ResultIndex, StorageUsage};
use std::cell::RefCell;
use std::cmp::min;
use std::collections::HashMap;

struct KV(Vec<u8>, Vec<u8>);

impl PartialEq for KV {
    fn eq(&self, other: &KV) -> bool {
        self.0.eq(&other.0)
    }
}

/// Mocked version of `Context`.
#[derive(Default)]
pub struct MockedContext {
    // We use vector instead of `BTreeMap` for internal representation of the trie, because of how
    // we iterate over it, we cannot use regular iterators and `BTreeMap` does not provide accessing
    // elements by index.
    data: RefCell<Vec<KV>>,
    next_iterator_id: RefCell<u32>,
    iterators: RefCell<HashMap<u32, (u32, Option<u32>)>>,
    results: RefCell<HashMap<ResultIndex, Vec<u8>>>,
    logs: RefCell<Vec<String>>,
    originator_id: RefCell<AccountId>,
    account_id: RefCell<AccountId>,
    frozen_balance: RefCell<Balance>,
    liquid_balance: RefCell<Balance>,
    received_amount: RefCell<Balance>,
    storage_usage: RefCell<StorageUsage>,
    block_index: RefCell<BlockIndex>,
}

impl MockedContext {
    pub fn new() -> Self {
        Self::default()
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

    pub fn set_account_id(&self, value: AccountId) {
        *self.account_id.borrow_mut() = value;
    }

    pub fn set_originator_id(&self, value: AccountId) {
        *self.originator_id.borrow_mut() = value;
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
        self.iterators.borrow_mut().insert(*self.next_iterator_id.borrow(), (from as _, None));
        *self.next_iterator_id.borrow_mut() += 1;
        *self.next_iterator_id.borrow() - 1
    }

    fn storage_range(&self, start: &[u8], end: &[u8]) -> u32 {
        let from = self.entry_unchecked(start);
        let to = Some(self.entry_unchecked(end) as u32);
        self.iterators.borrow_mut().insert(*self.next_iterator_id.borrow(), (from as _, to));
        *self.next_iterator_id.borrow_mut() += 1;
        *self.next_iterator_id.borrow() - 1
    }

    fn storage_iter_next(&self, iter_id: u32) -> bool {
        let (curr, end) = self.iterators.borrow()[&iter_id];
        if end.is_some() && end.unwrap() <= curr + 1 {
            self.iterators.borrow_mut().remove(&iter_id);
            return false;
        }
        match self.data.borrow().get(curr as usize) {
            Some(_el) => {
                self.iterators.borrow_mut().insert(iter_id, (curr + 1, end));
                true
            }
            None => {
                self.iterators.borrow_mut().remove(&iter_id);
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

    fn return_value(&self, _value: &[u8]) {
        unimplemented!()
    }

    fn return_promise(&self, _promise_index: u32) {
        unimplemented!()
    }

    /// This method should not be called by the mocked functions.
    fn data_read(
        &self,
        _data_type_index: u32,
        _key_len: usize,
        _key: u32,
        _max_buf_len: usize,
        _buf_ptr: *mut u8,
    ) -> usize {
        unimplemented!()
    }

    fn promise_create(
        &self,
        _account_id: &[u8],
        _method_name: &[u8],
        _arguments: &[u8],
        _amount: u64,
    ) -> u32 {
        unimplemented!()
    }

    fn promise_then(
        &self,
        _promise_index: u32,
        _method_name: &[u8],
        _arguments: &[u8],
        _amount: u64,
    ) -> u32 {
        unimplemented!()
    }

    fn promise_and(&self, _promise_index1: u32, _promise_index2: u32) -> u32 {
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
        *self.liquid_balance.borrow_mut() -= delta;
        *self.frozen_balance.borrow_mut() += delta;
        delta
    }

    fn withdraw(&self, min_amount: u64, max_amount: u64) -> u64 {
        if *self.frozen_balance.borrow() < min_amount {
            return 0;
        }
        let delta = min(*self.frozen_balance.borrow(), max_amount);
        *self.frozen_balance.borrow_mut() -= delta;
        *self.liquid_balance.borrow_mut() += delta;
        delta
    }

    fn received_amount(&self) -> u64 {
        *self.received_amount.borrow()
    }

    fn storage_usage(&self) -> u64 {
        *self.storage_usage.borrow()
    }

    fn assert(&self, expr: bool) {
        assert!(expr)
    }

    fn random_buf(&self, _buf: &mut [u8]) {
        unimplemented!()
    }

    fn random32(&self) -> u32 {
        unimplemented!()
    }

    fn block_index(&self) -> BlockIndex {
        *self.block_index.borrow()
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
        self.originator_id.borrow().clone()
    }

    fn account_id(&self) -> Vec<u8> {
        self.account_id.borrow().clone()
    }

    fn as_mock(&self) -> &MockedContext {
        self
    }
}
