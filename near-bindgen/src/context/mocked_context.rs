use crate::context::{AccountId, Balance, BlockIndex, Context, ResultIndex, StorageUsage};
use std::cell::RefCell;
use std::cmp::min;
use std::collections::HashMap;
use std::ops::{Deref, DerefMut};

struct KV(Vec<u8>, Vec<u8>);

impl PartialEq for KV {
    fn eq(&self, other: &KV) -> bool {
        self.0.eq(&other.0)
    }
}

#[derive(PartialEq, Eq, Clone, Debug)]
pub struct PromiseCreateEntry {
    pub account_id: Vec<u8>,
    pub method_name: Vec<u8>,
    pub arguments: Vec<u8>,
    pub amount: u64,
}

#[derive(PartialEq, Eq, Clone, Debug)]
pub struct PromiseThen {
    pub promise_index: u32,
    pub method_name: Vec<u8>,
    pub arguments: Vec<u8>,
    pub amount: u64,
}

#[derive(PartialEq, Eq, Clone, Debug)]
pub struct PromiseAnd {
    pub promise_index1: u32,
    pub promise_index2: u32,
}

#[derive(Default)]
pub struct MockedContextInternal {
    // We use vector instead of `BTreeMap` for internal representation of the trie, because of how
    // we iterate over it, we cannot use regular iterators and `BTreeMap` does not provide accessing
    // elements by index.
    data: Vec<KV>,
    next_iterator_id: u32,
    iterators: HashMap<u32, (u32, Option<u32>)>,
    results: HashMap<ResultIndex, Vec<u8>>,
    logs: Vec<String>,
    originator_id: AccountId,
    account_id: AccountId,
    frozen_balance: Balance,
    liquid_balance: Balance,
    received_amount: Balance,
    storage_usage: StorageUsage,
    block_index: BlockIndex,
    next_promise_id: u32,
    promise_create: Vec<PromiseCreateEntry>,
    promise_then: Vec<PromiseThen>,
    promise_and: Vec<PromiseAnd>,
    return_value: Vec<Vec<u8>>,
    return_promise: Vec<u32>,
}

/// Mocked version of `Context`.
#[derive(Default)]
pub struct MockedContext {
    internal: RefCell<MockedContextInternal>,
}

impl Deref for MockedContext {
    type Target = MockedContextInternal;

    fn deref(&self) -> &Self::Target {
        let ptr = self.internal.borrow().deref() as *const Self::Target;
        unsafe { &*ptr }
    }
}

impl MockedContext {
    fn borrow_mut(&self) -> &mut MockedContextInternal {
        let ptr = self.internal.borrow_mut().deref_mut() as *mut MockedContextInternal;
        unsafe { &mut *ptr }
    }
    pub fn new() -> Self {
        Self::default()
    }
    fn entry(&self, key: &[u8]) -> Result<usize, usize> {
        self.data.binary_search_by(|el| el.0.as_slice().cmp(key))
    }

    fn entry_unchecked(&self, key: &[u8]) -> usize {
        match self.entry(key) {
            Ok(ind) => ind,
            Err(ind) => ind,
        }
    }

    pub fn set_account_id(&self, value: AccountId) {
        self.borrow_mut().account_id = value;
    }

    pub fn set_originator_id(&self, value: AccountId) {
        self.borrow_mut().originator_id = value;
    }

    pub fn set_block_index(&self, value: BlockIndex) {
        self.borrow_mut().block_index = value;
    }

    pub fn get_promise_create(&self) -> Vec<PromiseCreateEntry> {
        self.promise_create.to_vec()
    }

    pub fn get_promise_then(&self) -> Vec<PromiseThen> {
        self.promise_then.to_vec()
    }

    pub fn get_promise_and(&self) -> Vec<PromiseAnd> {
        self.promise_and.to_vec()
    }
}

impl Context for MockedContext {
    fn storage_write(&self, key: &[u8], value: &[u8]) {
        let kv = KV(key.to_vec(), value.to_vec());
        match self.entry(key) {
            Ok(ind) => {
                *self.borrow_mut().data.get_mut(ind).unwrap() = kv;
            }
            Err(ind) => {
                self.borrow_mut().data.insert(ind, kv);
            }
        }
    }

    fn storage_iter(&self, prefix: &[u8]) -> u32 {
        let from = self.entry_unchecked(prefix);
        self.borrow_mut().iterators.insert(self.next_iterator_id, (from as _, None));
        self.borrow_mut().next_iterator_id += 1;
        self.next_iterator_id - 1
    }

    fn storage_range(&self, start: &[u8], end: &[u8]) -> u32 {
        let from = self.entry_unchecked(start);
        let to = Some(self.entry_unchecked(end) as u32);
        self.borrow_mut().iterators.insert(self.next_iterator_id, (from as _, to));
        self.borrow_mut().next_iterator_id += 1;
        self.next_iterator_id - 1
    }

    fn storage_iter_next(&self, iter_id: u32) -> bool {
        let (curr, end) = self.iterators[&iter_id];
        if end.is_some() && end.unwrap() <= curr + 1 {
            self.borrow_mut().iterators.remove(&iter_id);
            return false;
        }
        match self.data.get(curr as usize) {
            Some(_el) => {
                self.borrow_mut().iterators.insert(iter_id, (curr + 1, end));
                true
            }
            None => {
                self.borrow_mut().iterators.remove(&iter_id);
                false
            }
        }
    }

    fn storage_remove(&self, key: &[u8]) {
        self.entry(key).map(|ind| self.borrow_mut().data.remove(ind).1).unwrap();
    }

    fn storage_has_key(&self, key: &[u8]) -> bool {
        self.entry(key).map(|ind| self.data[ind].1.clone()).is_ok()
    }

    fn result_count(&self) -> u32 {
        self.results.len() as _
    }

    fn result_is_ok(&self, index: u32) -> bool {
        self.results.contains_key(&index)
    }

    fn return_value(&self, value: &[u8]) {
        self.borrow_mut().return_value.push(value.to_vec());
    }

    fn return_promise(&self, promise_index: u32) {
        self.borrow_mut().return_promise.push(promise_index);
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
        account_id: &[u8],
        method_name: &[u8],
        arguments: &[u8],
        amount: u64,
    ) -> u32 {
        self.borrow_mut().promise_create.push(PromiseCreateEntry{
            account_id: account_id.to_vec(),
            method_name: method_name.to_vec(),
            arguments: arguments.to_vec(),
            amount
        });
        self.borrow_mut().next_promise_id += 1;
        self.next_promise_id - 1
    }

    fn promise_then(
        &self,
        promise_index: u32,
        method_name: &[u8],
        arguments: &[u8],
        amount: u64,
    ) -> u32 {
        self.borrow_mut().promise_then.push(PromiseThen {
           promise_index,
            method_name: method_name.to_vec(),
            arguments: arguments.to_vec(),
            amount
        });
        self.borrow_mut().next_promise_id += 1;
        self.next_promise_id - 1
    }

    fn promise_and(&self, promise_index1: u32, promise_index2: u32) -> u32 {
        self.borrow_mut().promise_and.push(PromiseAnd {
            promise_index1,
            promise_index2
        });
        self.borrow_mut().next_promise_id += 1;
        self.next_promise_id - 1
    }

    fn frozen_balance(&self) -> u64 {
        self.frozen_balance
    }

    fn liquid_balance(&self) -> u64 {
        self.liquid_balance
    }

    fn deposit(&self, min_amount: u64, max_amount: u64) -> u64 {
        if self.liquid_balance < min_amount {
            return 0;
        }
        let delta = min(self.liquid_balance, max_amount);
        self.borrow_mut().liquid_balance -= delta;
        self.borrow_mut().frozen_balance += delta;
        delta
    }

    fn withdraw(&self, min_amount: u64, max_amount: u64) -> u64 {
        if self.frozen_balance < min_amount {
            return 0;
        }
        let delta = min(self.frozen_balance, max_amount);
        self.borrow_mut().frozen_balance -= delta;
        self.borrow_mut().liquid_balance += delta;
        delta
    }

    fn received_amount(&self) -> u64 {
        self.received_amount
    }

    fn storage_usage(&self) -> u64 {
        self.storage_usage
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
        self.block_index
    }

    fn debug(&self, msg: &[u8]) {
        let msg = String::from_utf8(msg.to_vec()).unwrap();
        self.borrow_mut().logs.push(msg);
    }

    fn storage_read(&self, key: &[u8]) -> Vec<u8> {
        self.entry(key).map(|ind| self.data[ind].1.clone()).unwrap_or_default()
    }

    fn storage_peek(&self, iter_id: u32) -> Vec<u8> {
        let (curr, end) = self.iterators[&iter_id];
        if end.is_some() && curr == end.unwrap() {
            return vec![];
        }
        self.data.get(curr as usize).map(|el| el.0.clone()).unwrap_or_default()
    }

    fn originator_id(&self) -> Vec<u8> {
        self.originator_id.clone()
    }

    fn account_id(&self) -> Vec<u8> {
        self.account_id.clone()
    }

    fn as_mock(&self) -> &MockedContext {
        self
    }
}
