//! Counting mock host for `state`. Not a stable API.
//!
//! `use_mock()` installs an in-memory host that records every read, write and remove with
//! its key and byte counts. The store persists across `begin_call()`s on the same thread, so
//! a test can model several calls; the receipt is per call.

use super::cache::{self, Host};
use std::cell::RefCell;
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OpKind {
    Read,
    Write,
    Remove,
    /// `storage_has_key`: charged per key byte, moves no value bytes.
    HasKey,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Op {
    pub kind: OpKind,
    pub key: Vec<u8>,
    /// Bytes read, written, or (for remove) the size of the value removed.
    pub value_len: usize,
    /// For a write, the size of the value it overwrote (the host charges for it).
    pub replaced_len: Option<usize>,
}

/// Host calls made in one call, in order.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Receipt {
    pub ops: Vec<Op>,
    /// sha256 computations (each one host call on chain) since `begin_call`.
    pub sha256: u64,
}

impl Receipt {
    fn count(&self, k: OpKind) -> usize {
        self.ops.iter().filter(|o| o.kind == k).count()
    }
    pub fn reads(&self) -> usize {
        self.count(OpKind::Read)
    }
    pub fn writes(&self) -> usize {
        self.count(OpKind::Write)
    }
    pub fn has_keys(&self) -> usize {
        self.count(OpKind::HasKey)
    }
    pub fn removes(&self) -> usize {
        self.count(OpKind::Remove)
    }
    /// `(reads, writes, removes)`
    pub fn rwd(&self) -> (usize, usize, usize) {
        (self.reads(), self.writes(), self.removes())
    }
    /// Value bytes moved by reads.
    pub fn read_bytes(&self) -> usize {
        self.ops.iter().filter(|o| o.kind == OpKind::Read).map(|o| o.value_len).sum()
    }
    /// Value bytes written, plus bytes of values overwritten or removed (all are charged).
    pub fn written_and_replaced_bytes(&self) -> usize {
        self.ops
            .iter()
            .filter(|o| o.kind != OpKind::Read)
            .map(|o| o.value_len + o.replaced_len.unwrap_or(0))
            .sum()
    }
    /// Largest value any single host call moved or replaced. The "local" guarantee says this
    /// never depends on a neighbor's value.
    pub fn max_value_bytes(&self) -> usize {
        self.ops.iter().map(|o| o.value_len.max(o.replaced_len.unwrap_or(0))).max().unwrap_or(0)
    }
    pub fn keys_written(&self) -> Vec<Vec<u8>> {
        self.ops.iter().filter(|o| matches!(o.kind, OpKind::Write | OpKind::Remove)).map(|o| o.key.clone()).collect()
    }
}

#[derive(Default)]
struct MockState {
    store: BTreeMap<Vec<u8>, Vec<u8>>,
    receipt: Receipt,
    sha_at_begin: u64,
}

std::thread_local! {
    static MOCK: RefCell<MockState> = RefCell::new(MockState::default());
}

struct MockHost;

impl Host for MockHost {
    fn read(&mut self, key: &[u8]) -> Option<Vec<u8>> {
        MOCK.with(|m| {
            let mut m = m.borrow_mut();
            let v = m.store.get(key).cloned();
            m.receipt.ops.push(Op {
                kind: OpKind::Read,
                key: key.to_vec(),
                value_len: v.as_ref().map_or(0, Vec::len),
                replaced_len: None,
            });
            v
        })
    }
    fn write(&mut self, key: &[u8], value: &[u8]) {
        MOCK.with(|m| {
            let mut m = m.borrow_mut();
            let old = m.store.insert(key.to_vec(), value.to_vec());
            m.receipt.ops.push(Op {
                kind: OpKind::Write,
                key: key.to_vec(),
                value_len: value.len(),
                replaced_len: old.map(|o| o.len()),
            });
        })
    }
    fn has_key(&mut self, key: &[u8]) -> bool {
        MOCK.with(|m| {
            let mut m = m.borrow_mut();
            let present = m.store.contains_key(key);
            m.receipt.ops.push(Op {
                kind: OpKind::HasKey,
                key: key.to_vec(),
                value_len: 0,
                replaced_len: None,
            });
            present
        })
    }
    fn remove(&mut self, key: &[u8]) {
        MOCK.with(|m| {
            let mut m = m.borrow_mut();
            let old = m.store.remove(key);
            m.receipt.ops.push(Op {
                kind: OpKind::Remove,
                key: key.to_vec(),
                value_len: old.map_or(0, |o| o.len()),
                replaced_len: None,
            });
        })
    }
}

/// Installs a fresh counting mock with an empty store, and starts a call.
pub fn use_mock() {
    MOCK.with(|m| *m.borrow_mut() = MockState::default());
    cache::install_host(Box::new(MockHost));
    begin_call();
}

/// Routes the cache to `env::storage_*` (the mocked blockchain under `unit-testing`).
pub fn use_env_host() {
    cache::install_host(Box::new(cache::EnvHost));
}

/// Starts a new call: empty cache, empty receipt. The store persists.
pub fn begin_call() {
    cache::begin_call();
    MOCK.with(|m| {
        let mut m = m.borrow_mut();
        m.receipt = Receipt::default();
        m.sha_at_begin = super::key::sha_calls();
    });
}

/// Flushes the cache without a root record (for collection tests that have no root).
pub fn flush() {
    cache::flush(None);
}

/// The receipt of the current call so far.
pub fn receipt() -> Receipt {
    MOCK.with(|m| {
        let m = m.borrow();
        let mut r = m.receipt.clone();
        r.sha256 = super::key::sha_calls() - m.sha_at_begin;
        r
    })
}

/// Flushes, returns this call's receipt, and starts the next call.
pub fn end_call() -> Receipt {
    flush();
    let r = receipt();
    begin_call();
    r
}

/// Panics if the current call has anything to write (the view-call rule).
pub fn assert_view_clean() {
    assert!(!cache::has_pending_writes(), "state: a view call has pending writes");
}

/// Snapshot of the mock store.
pub fn store() -> BTreeMap<Vec<u8>, Vec<u8>> {
    MOCK.with(|m| m.borrow().store.clone())
}

/// Writes directly into the mock store, bypassing the cache (to seed fixtures).
pub fn seed(key: &[u8], value: &[u8]) {
    MOCK.with(|m| {
        m.borrow_mut().store.insert(key.to_vec(), value.to_vec());
    });
}

/// Number of slots the cache holds in this call.
pub fn cache_slots() -> usize {
    cache::slot_count()
}
