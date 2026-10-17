//! Test support for `near_sdk::state` contracts: a counting mock host and a small harness.
//!
//! `use_mock()` installs an in-memory host that records every read, write, remove and
//! `storage_has_key` with its key and byte counts. The store persists across `begin_call()`s
//! on the same thread, so a test can model several calls; the [`Receipt`] is per call.
//!
//! [`Harness`] does what the `#[near]` wrapper does around each method:
//!
//! ```ignore
//! let (h, _) = Harness::init("owner.near", || Exchange::new("owner.near".parse().unwrap()));
//! let (_, r) = h.call("alice.near", NearToken::from_yoctonear(1), |c| c.deposit("usdt".into(), U128(5)));
//! assert_eq!(r.rwd(), (3, 2, 0)); // reads, writes, removes in that call
//! let (bal, _) = h.view(|c| c.balance_of("alice.near".parse().unwrap(), "usdt".into()));
//! ```
//!
//! Two traps: only the mock produces receipts, and only `use_env_host()` (the mocked
//! blockchain) tracks `env::storage_usage()`, so a storage-bond measurement needs the env host.
//! And `testing_env!` resets `storage_usage` from its builder, so measure after the context
//! switch.

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

// ---- one ordered log of every `env::storage_*` call (migration tests) ----
//
// The counting mock above only sees the v2 cache. v1 collections (`store`, `collections`) call
// `env::storage_*` directly, so a test that runs v1 and v2 code over the same storage uses
// `use_env_host()` (the mocked blockchain) and this log, which `env.rs` feeds from all four
// storage host functions. Off by default.

std::thread_local! {
    static ENV_LOG: RefCell<Option<Vec<Op>>> = const { RefCell::new(None) };
}

#[doc(hidden)]
pub fn __env_log_on() -> bool {
    ENV_LOG.with(|l| l.borrow().is_some())
}

#[doc(hidden)]
pub fn __env_log(kind: OpKind, key: &[u8], value_len: usize, replaced_len: Option<usize>) {
    ENV_LOG.with(|l| {
        if let Some(log) = l.borrow_mut().as_mut() {
            log.push(Op { kind, key: key.to_vec(), value_len, replaced_len });
        }
    })
}

/// Starts (or restarts) recording every `env::storage_*` call, v1 and v2 alike, in order.
pub fn env_log_start() {
    ENV_LOG.with(|l| *l.borrow_mut() = Some(Vec::new()));
}

/// The calls recorded since `env_log_start` (or the last take), as a [`Receipt`]; keeps
/// recording. `sha256` is not counted here.
pub fn env_log_take() -> Receipt {
    ENV_LOG.with(|l| Receipt { ops: l.borrow_mut().as_mut().map(core::mem::take).unwrap_or_default(), sha256: 0 })
}

/// Stops recording.
pub fn env_log_stop() {
    ENV_LOG.with(|l| *l.borrow_mut() = None);
}

/// Snapshot of the mocked blockchain's storage (what `env::storage_*` sees), sorted by key.
#[cfg(feature = "unit-testing")]
pub fn env_store() -> BTreeMap<Vec<u8>, Vec<u8>> {
    crate::mock::with_mocked_blockchain(|b| b.storage_snapshot()).into_iter().collect()
}

/// Sets the mocked blockchain context: predecessor (and signer) `who`, attached `deposit`.
#[cfg(feature = "unit-testing")]
pub fn set_context(who: &str, deposit: crate::NearToken) {
    let who: crate::AccountId = who.parse().expect("valid account id");
    let ctx = crate::test_utils::VMContextBuilder::new()
        .predecessor_account_id(who.clone())
        .signer_account_id(who)
        .attached_deposit(deposit)
        .build();
    crate::testing_env!(ctx);
}

/// Runs contract methods the way the `#[near]` wrapper does, against the counting mock:
/// read the root, run the method, write the root (for `call`), and return the call's
/// [`Receipt`]. The store persists across calls on the same thread.
#[cfg(feature = "unit-testing")]
pub struct Harness<T> {
    _m: core::marker::PhantomData<fn() -> T>,
}

#[cfg(feature = "unit-testing")]
impl<T> Harness<T>
where
    T: super::ContractState + super::StateRead + super::StateWrite,
{
    /// Installs a fresh counting mock (empty store), runs `init` as `who` with no deposit,
    /// and writes the returned root, as an `#[init]` call does.
    pub fn init(who: &str, init: impl FnOnce() -> T) -> (Self, Receipt) {
        set_context(who, crate::NearToken::from_yoctonear(0));
        use_mock();
        let root = init();
        root.state_write();
        drop(root);
        (Harness { _m: core::marker::PhantomData }, receipt())
    }

    /// A harness over whatever the current host and store hold (no reset).
    pub fn attach() -> Self {
        Harness { _m: core::marker::PhantomData }
    }

    /// A mutating call as `who` with `deposit` attached. The root is written after `f`
    /// returns and dropped after the write, as in the wrapper.
    pub fn call<R>(
        &self,
        who: &str,
        deposit: crate::NearToken,
        f: impl FnOnce(&mut T) -> R,
    ) -> (R, Receipt) {
        set_context(who, deposit);
        begin_call();
        let mut root = T::state_read().expect("state: contract is not initialized");
        let r = f(&mut root);
        root.state_write();
        drop(root);
        (r, receipt())
    }

    /// A view call: nothing may be pending at the end ([`assert_view_clean`]). Keeps the
    /// current context (predecessor, deposit).
    pub fn view<R>(&self, f: impl FnOnce(&T) -> R) -> (R, Receipt) {
        begin_call();
        let root = T::state_read().expect("state: contract is not initialized");
        let r = f(&root);
        assert_view_clean();
        drop(root);
        (r, receipt())
    }
}
