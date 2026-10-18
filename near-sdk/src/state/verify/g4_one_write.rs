//! Guarantee 4, One write per key per call: writes land at the end of the method, once per
//! modified key, in key order with the root last; view methods never write; nothing happens in
//! `Drop` for v2 types.
//!
//! Checked as INVARIANTS on the real receipts (independent of the model): every generated run
//! through the differential driver (which also asserts that no host write or remove happens
//! before the flush), and a Kani harness directly on the cache with a recording host.

use super::diff::{CallKind, ops_gen, run};
use super::fixture::MRoot;
use crate::state::model::{Model, Op};
use crate::state::testing::{self, OpKind};
use crate::state::{flush_root, load_root};

/// The invariant on one call's host calls: writes and removes touch each key at most once, in
/// strictly ascending key order, except the root, which may only come last.
pub fn assert_one_write_per_key(ops: &[testing::Op], root: &[u8]) {
    let muts: std::vec::Vec<&testing::Op> =
        ops.iter().filter(|o| matches!(o.kind, OpKind::Write | OpKind::Remove)).collect();
    let (body, last) = match muts.split_last() {
        Some((l, b)) if l.key == root => (b, Some(*l)),
        _ => (&muts[..], None),
    };
    for w in body.windows(2) {
        assert!(w[0].key < w[1].key, "flush out of key order or a key written twice: {:?} then {:?}", w[0].key, w[1].key);
    }
    assert!(body.iter().all(|o| o.key != root), "the root was written before other keys");
    if let Some(l) = last {
        assert_eq!(l.kind, OpKind::Write, "the root is removed");
    }
}

/// Random engine, 2,000 runs of up to 60 operations (the Part A generator): every mutating
/// call and every mid-call flush segment satisfies the invariant; every view call has no write
/// or remove at all; no host write precedes the flush (asserted inside the driver).
#[test]
fn g4_one_write_per_key_per_call_in_key_order_root_last() {
    let root = crate::state::model::root_key();
    bolero::check!().with_generator(ops_gen()).with_iterations(2_000).for_each(|ops| {
        for log in run(ops) {
            match log.kind {
                CallKind::Mutating | CallKind::MidFlush => assert_one_write_per_key(&log.ops, &root),
                CallKind::View => assert!(
                    log.ops.iter().all(|o| matches!(o.kind, OpKind::Read | OpKind::HasKey)),
                    "a view call wrote"
                ),
            }
        }
    });
}

/// Nothing in `Drop`: a call that runs any operations and then drops the root and every handle
/// WITHOUT flushing (a panicking call) makes no host write or remove, and storage is unchanged,
/// also after the next call starts and frees the cache. Random engine, 2,000 cases.
#[test]
fn g4_nothing_happens_in_drop() {
    let g = (ops_gen(), ops_gen());
    bolero::check!().with_generator(g).with_iterations(2_000).for_each(|(setup, body)| {
        let (_, st) = super::diff::run_state(setup);
        let before = testing::store();
        // the model over the stored state is only the legality oracle for the body here
        let mut m = Model { committed: st.clone(), cur: st, cache: crate::state::model::CacheModel::new(before.clone()) };
        testing::begin_call();
        let mut root = load_root::<MRoot>();
        for op in body.iter().filter(|o| !matches!(o, Op::EndCall | Op::MidFlush | Op::View(_) | Op::RecRetyped(..))) {
            if m.would_panic(op) {
                continue;
            }
            m.apply(op);
            super::diff::real_apply(&mut root, op);
        }
        drop(root);
        crate::state::cache::begin_call();
        let r = testing::receipt();
        assert!(r.ops.iter().all(|o| matches!(o.kind, OpKind::Read | OpKind::HasKey)), "a write happened without a flush");
        assert_eq!(testing::store(), before, "storage changed without a flush");
    });
}

/// The root is written last even when it sorts first (its key `F0` is a prefix of every key).
#[test]
fn g4_root_is_last_although_its_key_sorts_first() {
    testing::use_mock();
    let mut root = load_root::<MRoot>();
    root.counter = 5;
    root.flat.insert(1, vec![1]);
    root.vec.push(vec![2]);
    flush_root(&root);
    drop(root);
    let r = testing::receipt();
    let w: std::vec::Vec<_> = r.ops.iter().filter(|o| o.kind == OpKind::Write).map(|o| o.key.clone()).collect();
    assert_eq!(w.last().unwrap(), &crate::state::model::root_key());
    assert!(w[..w.len() - 1].iter().all(|k| k.as_slice() > crate::state::model::root_key().as_slice()));
}

// ---------------------------------------------------------------------------------------
// Kani: the cache itself, with a recording host
// ---------------------------------------------------------------------------------------

use crate::state::cache::{self, Host};
use crate::state::key::{DEFAULT_ROOT, Path};
use bolero::generator::TypeGenerator;
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Default)]
struct Rec {
    store: std::vec::Vec<(std::vec::Vec<u8>, std::vec::Vec<u8>)>,
    log: std::vec::Vec<(OpKind, std::vec::Vec<u8>)>,
}
struct RecHost(Rc<RefCell<Rec>>);
impl Host for RecHost {
    fn read(&mut self, key: &[u8]) -> Option<std::vec::Vec<u8>> {
        let mut r = self.0.borrow_mut();
        r.log.push((OpKind::Read, key.to_vec()));
        r.store.iter().find(|(k, _)| k == key).map(|(_, v)| v.clone())
    }
    fn has_key(&mut self, key: &[u8]) -> bool {
        let mut r = self.0.borrow_mut();
        r.log.push((OpKind::HasKey, key.to_vec()));
        r.store.iter().any(|(k, _)| k == key)
    }
    fn write(&mut self, key: &[u8], value: &[u8]) {
        let mut r = self.0.borrow_mut();
        r.log.push((OpKind::Write, key.to_vec()));
        r.store.retain(|(k, _)| k != key);
        r.store.push((key.to_vec(), value.to_vec()));
    }
    fn remove(&mut self, key: &[u8]) {
        let mut r = self.0.borrow_mut();
        r.log.push((OpKind::Remove, key.to_vec()));
        r.store.retain(|(k, _)| k != key);
    }
}

/// One cache operation on one of three keys (all distinct from the root key `F0`).
#[derive(Clone, Copy, Debug, TypeGenerator)]
pub enum CacheOp {
    Get(#[generator(0..3)] u8),
    Exists(#[generator(0..3)] u8),
    Put(#[generator(0..3)] u8, #[generator(0..2)] u8),
    Delete(#[generator(0..3)] u8),
    /// `get` then mutate through `&mut` (touch), possibly to the same value.
    Touch(#[generator(0..3)] u8, #[generator(0..2)] u8),
}

fn key(i: u8) -> Path {
    Path::root(DEFAULT_ROOT).field(1 + i as u32)
}

/// G4 on the cache, model-checked. Bound (Kani): 3 keys, each initially absent or holding
/// one of two values; up to 6 symbolic cache operations (get, exists, put, delete, get+mutate)
/// then a flush with a root record. Each key is written or removed at most once, in key order,
/// the root last; nothing is written before the flush; a run of only gets and exists writes
/// nothing (the view rule). Random engine: 20,000 sequences.
///
/// Kani: NOT RUN. Kani 0.68.0 (pinned nightly-2026-08-21) panics compiling any reachable
/// `catch_unwind` (its intrinsic signature check expects an `i32` return; minimal repro: a
/// harness calling `std::panic::catch_unwind(|| 1)`). The cache's `dyn` values make the
/// `unit-testing` mock (enabled in `--tests` builds by near-sdk's self dev-dependency)
/// reachable, and with it `catch_unwind`; and because Kani compiles every proof harness in the
/// crate together, a `kani::proof` here would break every other Kani run. Restore the
/// attributes once Kani accepts the current `catch_unwind` signature.
#[test]
fn g4_cache_flush_writes_each_key_once_in_order_root_last() {
    let g = (bolero::produce::<[Option<u8>; 3]>(), bolero::produce::<[CacheOp; 6]>(), 0usize..=6, bolero::produce::<bool>());
    bolero::check!().with_generator(g).with_iterations(20_000).for_each(|(init, ops, n, root_changes)| {
        let rec = Rc::new(RefCell::new(Rec::default()));
        for (i, v) in init.iter().enumerate() {
            if let Some(v) = v {
                rec.borrow_mut().store.push((key(i as u8).as_bytes().to_vec(), vec![v % 2]));
            }
        }
        rec.borrow_mut().store.push((vec![DEFAULT_ROOT], vec![0]));
        cache::install_host(Box::new(RecHost(rec.clone())));
        let root = Path::root(DEFAULT_ROOT);
        let _ = cache::read_root(&root);
        let mut view_only = true;
        for op in &ops[..*n] {
            match *op {
                CacheOp::Get(i) => {
                    let _ = cache::get::<u8>(&key(i));
                }
                CacheOp::Exists(i) => {
                    let _ = cache::exists(&key(i));
                }
                CacheOp::Put(i, v) => {
                    view_only = false;
                    cache::put(&key(i), v);
                }
                CacheOp::Delete(i) => {
                    view_only = false;
                    let _ = cache::delete::<u8>(&key(i));
                }
                CacheOp::Touch(i, v) => {
                    if let Some(p) = cache::get::<u8>(&key(i)) {
                        view_only = false;
                        // SAFETY: no other reference to this slot's value is alive
                        unsafe { *p.as_ptr() = v };
                        cache::touch(&key(i));
                    }
                }
            }
        }
        assert!(
            rec.borrow().log.iter().all(|(k, _)| matches!(k, OpKind::Read | OpKind::HasKey)),
            "host write before the flush"
        );
        let mark = rec.borrow().log.len();
        let new_root = if *root_changes { vec![1u8] } else { vec![0u8] };
        cache::flush(Some((&root, &|_| new_root.clone())));
        let r = rec.borrow();
        let muts: std::vec::Vec<&(OpKind, std::vec::Vec<u8>)> = r.log[mark..].iter().collect();
        let (body, last) = match muts.split_last() {
            Some((l, b)) if l.1 == [DEFAULT_ROOT] => (b.to_vec(), Some(*l)),
            _ => (muts.clone(), None),
        };
        for w in body.windows(2) {
            assert!(w[0].1 < w[1].1, "flush out of order or a key twice");
        }
        assert!(body.iter().all(|(_, k)| k.as_slice() != [DEFAULT_ROOT]));
        assert_eq!(last.is_some(), *root_changes, "root written iff its bytes changed");
        if view_only {
            assert!(body.is_empty(), "a read-only sequence wrote");
        }
    });
}
