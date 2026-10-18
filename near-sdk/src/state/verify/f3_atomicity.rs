//! F3, receipt failure atomicity. A receipt that panics is reverted by the runtime. The SDK's
//! side of the contract: no in-memory state (cache slots, clearing flags, pending values)
//! survives into the next call, so a history with a failed call behaves exactly as the same
//! history without that call. A host that panics at the i-th write/remove of a chosen call's
//! flush (every flush position: first write, middle, the root write last, or after it), or a
//! panic in the method body before the flush, is followed by a simulated revert (storage
//! restored to its snapshot); the model drops that call (`abort_call`). After every call the
//! store equals the model's, and every operation's result agrees.
//!
//! Legacy `store::*` drop writes: on chain a panic traps without unwinding, so a v1
//! collection's write-on-drop never runs in a failed receipt, and the revert covers the v2
//! flush before it. In the unwinding mock the drop DOES run during the unwind; the revert
//! still restores the snapshot, which `c3`/`state_v2_migrate` cover for the ordering itself.

use super::diff::{ops_gen, real_apply};
use super::fixture::MRoot;
use crate::state::cache::{self, Host};
use crate::state::model::{Model, Op};
use crate::state::{flush_root, load_root};
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::rc::Rc;
use std::sync::atomic::{AtomicUsize, Ordering};

static FLUSH_FAILS: AtomicUsize = AtomicUsize::new(0);
static BODY_FAILS: AtomicUsize = AtomicUsize::new(0);

#[derive(Default)]
struct St {
    store: BTreeMap<Vec<u8>, Vec<u8>>,
    muts: usize,
    fail_at: Option<usize>,
}

struct FailHost(Rc<RefCell<St>>);
impl FailHost {
    fn tick(&self) {
        let mut s = self.0.borrow_mut();
        let n = s.muts;
        s.muts += 1;
        if s.fail_at == Some(n) {
            drop(s);
            panic!("injected host failure at mutation {n}");
        }
    }
}
impl Host for FailHost {
    fn read(&mut self, key: &[u8]) -> Option<Vec<u8>> {
        self.0.borrow().store.get(key).cloned()
    }
    fn has_key(&mut self, key: &[u8]) -> bool {
        self.0.borrow().store.contains_key(key)
    }
    fn write(&mut self, key: &[u8], value: &[u8]) {
        self.tick();
        self.0.borrow_mut().store.insert(key.to_vec(), value.to_vec());
    }
    fn remove(&mut self, key: &[u8]) {
        self.tick();
        self.0.borrow_mut().store.remove(key);
    }
}

fn quiet<R>(f: impl FnOnce() -> R) -> std::thread::Result<R> {
    let hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));
    let r = catch_unwind(AssertUnwindSafe(f));
    std::panic::set_hook(hook);
    r
}

/// Where the chosen call fails.
#[derive(Clone, Copy, Debug, bolero::generator::TypeGenerator)]
pub enum Fail {
    /// At the i-th host mutation of the flush (beyond the last: no failure).
    Flush(#[generator(0..12)] u8),
    /// In the method body after the j-th operation (no flush at all).
    Body(#[generator(0..8)] u8),
}

/// Random engine, 2,000 histories of up to 60 operations; up to 3 calls fail, each at a
/// random flush position or body position.
#[test]
fn f3_a_failed_call_leaves_no_trace() {
    let g = (ops_gen(), bolero::produce::<[(u8, Fail); 3]>());
    bolero::check!().with_generator(g).with_iterations(2_000).for_each(|(ops, fails)| {
        let st = Rc::new(RefCell::new(St::default()));
        cache::install_host(Box::new(FailHost(st.clone())));
        let mut m = Model::default();
        let calls: Vec<&[Op]> = ops.split(|o| matches!(o, Op::EndCall | Op::View(_) | Op::RecRetyped(..))).collect();
        for (ci, call) in calls.iter().enumerate() {
            let fail = fails.iter().find(|(c, _)| *c as usize % calls.len().max(1) == ci).map(|(_, f)| *f);
            let snapshot = st.borrow().store.clone();
            {
                let mut s = st.borrow_mut();
                s.muts = 0;
                s.fail_at = match fail {
                    Some(Fail::Flush(i)) => Some(i as usize),
                    _ => None,
                };
            }
            cache::begin_call();
            m.begin_call();
            let body: Vec<&Op> = call.iter().filter(|o| !matches!(o, Op::MidFlush)).collect();
            let r = quiet(|| {
                let mut root = load_root::<MRoot>();
                for (j, op) in body.iter().enumerate() {
                    if matches!(fail, Some(Fail::Body(b)) if b as usize == j) {
                        panic!("injected body failure");
                    }
                    if m.would_panic(op) {
                        continue;
                    }
                    assert_eq!(real_apply(&mut root, op), m.apply(op), "result differs at {op:?}");
                }
                if matches!(fail, Some(Fail::Body(b)) if b as usize >= body.len()) {
                    panic!("injected body failure just before the flush");
                }
                flush_root(&root);
            });
            match r {
                Ok(()) => {
                    let _ = m.end_call();
                }
                Err(e) => {
                    let msg = e.downcast_ref::<String>().cloned().or_else(|| e.downcast_ref::<&str>().map(|s| s.to_string())).unwrap_or_default();
                    assert!(msg.contains("injected"), "unexpected panic: {msg}");
                    let c = if msg.contains("host") { &FLUSH_FAILS } else { &BODY_FAILS };
                    c.fetch_add(1, Ordering::Relaxed);
                    st.borrow_mut().store = snapshot; // the runtime reverts the receipt
                    let _ = m.abort_call();
                }
            }
            st.borrow_mut().fail_at = None;
            assert_eq!(st.borrow().store, m.cache.host, "storage differs from the model after call {ci} ({fail:?})");
        }
    });
    let (f, b) = (FLUSH_FAILS.load(Ordering::Relaxed), BODY_FAILS.load(Ordering::Relaxed));
    println!("f3: {f} calls failed in the flush, {b} in the body");
    assert!(f > 20 && b > 20, "too few injected failures: {f} flush, {b} body");
}
