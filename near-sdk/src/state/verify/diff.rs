//! Part A: the differential driver. One generated operation sequence runs through the real
//! implementation (the `MRoot` fixture over the counting mock, the way the `#[near]` wrapper
//! drives a root) and through [`Model`]. After every operation the returned values must be
//! equal; after every call the host calls must equal the model's prediction (lookups as a
//! multiset, writes and removes in order, with key bytes and value lengths) and the mock's
//! storage must equal the model's storage byte for byte.

use super::fixture::{MRoot, recs_v2};
use crate::state::model::{Model, Op, Predicted, Ret};
use crate::state::testing::{self, OpKind, Receipt};
use crate::state::{flush_root, load_root};

/// What one call did on the real side, for invariant harnesses built on top of the driver.
#[derive(Clone, Debug)]
pub struct CallLog {
    pub kind: CallKind,
    /// Host calls of this call (or of this mid-call flush segment).
    pub ops: Vec<testing::Op>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CallKind {
    Mutating,
    /// Ended by a hand-written `state_write()` mid-method; the method continues.
    MidFlush,
    View,
}

pub fn real_apply(root: &mut MRoot, op: &Op) -> Ret {
    match *op {
        Op::EndCall | Op::MidFlush | Op::View(_) | Op::RecRetyped(..) => unreachable!(),
        Op::Counter(x) => {
            root.counter = root.counter.wrapping_add(x as u64);
            Ret::Unit
        }
        Op::FlatInsert(k, v) => Ret::Bool(root.flat.insert(k.0, v.bytes())),
        Op::FlatTryInsert(k, v) => Ret::Bool(root.flat.try_insert(k.0, v.bytes()).is_ok()),
        Op::FlatGet(k) => Ret::Bytes(root.flat.get(&k.0).cloned()),
        Op::FlatContains(k) => Ret::Bool(root.flat.contains_key(&k.0)),
        Op::FlatGetMut(k, x) => Ret::Bytes(root.flat.get_mut(&k.0).map(|v| {
            if let Some(b) = x {
                v.push(b);
            }
            v.clone()
        })),
        Op::FlatEntry(k, v) => Ret::Bytes(Some(root.flat.entry(k.0).or_insert(v.bytes()).clone())),
        Op::FlatRemove(k) => Ret::Bytes(root.flat.remove(&k.0)),
        Op::FlatDiscard(k) => Ret::Bool(root.flat.discard(&k.0)),
        Op::FlatPage(after, limit) => Ret::Page(
            root.flat
                .page(after.map(|k| k.0).as_ref(), limit as u32)
                .map(|p| (p.items, p.next, p.revision))
                .map_err(|_| ()),
        ),
        Op::FlatKeysPage(after, limit) => Ret::KeyPage(
            root.flat
                .keys_page(after.map(|k| k.0).as_ref(), limit as u32)
                .map(|p| (p.keys, p.next, p.revision))
                .map_err(|_| ()),
        ),
        Op::FlatIter(skip, take) => Ret::Items(
            root.flat.iter().skip(skip as usize).take(take as usize).map(|(k, v)| (k, v.clone())).collect(),
        ),
        Op::FlatLen => Ret::U32(root.flat.len()),
        Op::LookInsert(k, v) => {
            root.look.insert(k.0, v.bytes());
            Ret::Unit
        }
        Op::LookTryInsert(k, v) => Ret::Bool(root.look.try_insert(k.0, v.bytes()).is_ok()),
        Op::LookGet(k) => Ret::Bytes(root.look.get(&k.0).cloned()),
        Op::LookContains(k) => Ret::Bool(root.look.contains_key(&k.0)),
        Op::LookGetMut(k, x) => Ret::Bytes(root.look.get_mut(&k.0).map(|v| {
            if let Some(b) = x {
                v.push(b);
            }
            v.clone()
        })),
        Op::LookEntry(k, v) => Ret::Bytes(Some(root.look.entry(k.0).or_insert(v.bytes()).clone())),
        Op::LookRemove(k) => Ret::Bytes(root.look.remove(&k.0)),
        Op::LookDiscard(k) => {
            root.look.discard(&k.0);
            Ret::Unit
        }
        Op::RecTryInsert(k, n) => {
            Ret::Bool(root.recs.try_insert(k.0, super::fixture::Rec { n: n as u64, ..Default::default() }).is_ok())
        }
        Op::RecEntryDefault(k) => Ret::U64(root.recs.entry(k.0).or_default().n),
        Op::RecGetMut(k, x) => Ret::OptU64(root.recs.get_mut(&k.0).map(|r| {
            r.n = r.n.wrapping_add(x as u64);
            r.n
        })),
        Op::RecGet(k) => Ret::OptU64(root.recs.get(&k.0).map(|r| r.n)),
        Op::RecContains(k) => Ret::Bool(root.recs.contains_key(&k.0)),
        Op::RecAtMutInsert(k, k2, v) => match root.recs.at_mut(&k.0) {
            None => Ret::OptU64(None),
            Some(r) => Ret::Bool(r.items.insert(k2.0, v.bytes())),
        },
        Op::RecAtMutRemove(k, k2) => match root.recs.at_mut(&k.0) {
            None => Ret::OptU64(None),
            Some(r) => Ret::Bytes(r.items.remove(&k2.0)),
        },
        Op::RecAtLen(k) => Ret::U32(root.recs.at(&k.0).items.len()),
        Op::RecRemove(k) => Ret::OptU64(root.recs.remove(&k.0).map(|r| r.n)),
        Op::RecRemoveSome(k, n) => Ret::Bool(root.recs.remove_step(&k.0, n as u32) == crate::state::Removal::Pending),
        Op::RecPage(after, limit) => Ret::RecPage(
            root.recs
                .page_with(after.map(|k| k.0).as_ref(), limit as u32, |_, r| r.n)
                .map(|p| (p.items, p.next, p.revision))
                .map_err(|_| ()),
        ),
        Op::VecPush(v) => {
            root.vec.push(v.bytes());
            Ret::Unit
        }
        Op::VecPop => Ret::Bytes(root.vec.pop()),
        Op::VecSwapRemove(i) => Ret::Bytes(Some(root.vec.swap_remove(i as u32))),
        Op::VecGet(i) => Ret::Bytes(root.vec.get(i as u32).cloned()),
        Op::VecGetMut(i, x) => Ret::Bytes(root.vec.get_mut(i as u32).map(|v| {
            if let Some(b) = x {
                v.push(b);
            }
            v.clone()
        })),
        Op::VecLen => Ret::U32(root.vec.len()),
    }
}

fn split(ops: &[testing::Op]) -> Predicted {
    let mut p = Predicted::default();
    for o in ops {
        match o.kind {
            OpKind::Read | OpKind::HasKey => p.lookups.push((o.kind, o.key.clone(), o.value_len)),
            OpKind::Write | OpKind::Remove => {
                p.mutations.push((o.kind, o.key.clone(), o.value_len, o.replaced_len))
            }
        }
    }
    p
}

fn sorted(mut v: Vec<(OpKind, Vec<u8>, usize)>) -> Vec<(OpKind, Vec<u8>, usize)> {
    v.sort_by(|a, b| (a.0 as u8, &a.1, a.2).cmp(&(b.0 as u8, &b.1, b.2)));
    v
}

#[track_caller]
fn compare(what: &str, ops: &[testing::Op], mut pred: Predicted, trace: &[Op]) {
    let got = split(ops);
    pred.lookups = sorted(pred.lookups);
    let got_lookups = sorted(got.lookups);
    assert_eq!(got_lookups, pred.lookups, "{what}: reads / has_keys differ from the model\ntrace: {trace:?}");
    assert_eq!(got.mutations, pred.mutations, "{what}: writes / removes differ from the model\ntrace: {trace:?}");
}

/// Runs `ops` through both sides and compares everything. Returns the real side's per-call
/// host calls, for invariant harnesses. Operations the model says panic are skipped.
pub fn run(ops: &[Op]) -> Vec<CallLog> {
    run_state(ops).0
}

/// As [`run`], also returning the model's committed state at the end (storage equals its
/// `concretize()`).
pub fn run_state(ops: &[Op]) -> (Vec<CallLog>, crate::state::model::MState) {
    testing::use_mock();
    let mut m = Model::default();
    let mut logs = Vec::new();
    let mut trace: Vec<Op> = Vec::new();

    let begin = |m: &mut Model| -> MRoot {
        testing::begin_call();
        m.begin_call();
        load_root::<MRoot>()
    };
    let mut seg_start = 0usize;
    let end = |m: &mut Model, root: MRoot, seg_start: &mut usize, logs: &mut Vec<CallLog>, trace: &[Op]| {
        let before = testing::receipt();
        assert!(
            before.ops[*seg_start..].iter().all(|o| matches!(o.kind, OpKind::Read | OpKind::HasKey)),
            "a host write or remove happened before the flush\ntrace: {trace:?}"
        );
        flush_root(&root);
        drop(root);
        let r = testing::receipt();
        let seg = r.ops[*seg_start..].to_vec();
        compare("call", &seg, m.end_call(), trace);
        assert_eq!(testing::store(), m.cache.host, "storage bytes differ from the model\ntrace: {trace:?}");
        logs.push(CallLog { kind: CallKind::Mutating, ops: seg });
        *seg_start = 0;
    };

    let mut root = begin(&mut m);
    for op in ops {
        trace.push(op.clone());
        match *op {
            Op::EndCall => {
                end(&mut m, root, &mut seg_start, &mut logs, &trace);
                root = begin(&mut m);
            }
            Op::MidFlush => {
                flush_root(&root);
                m.mid_flush();
                let r = testing::receipt();
                let seg = r.ops[seg_start..].to_vec();
                compare("mid-call flush", &seg, m.cache.take_predicted(), &trace);
                assert_eq!(testing::store(), m.cache.host, "storage differs after a mid-call flush\ntrace: {trace:?}");
                seg_start = r.ops.len();
                logs.push(CallLog { kind: CallKind::MidFlush, ops: seg });
            }
            Op::View(k) => {
                end(&mut m, root, &mut seg_start, &mut logs, &trace);
                testing::begin_call();
                let v = load_root::<MRoot>();
                let got = Ret::View(
                    v.flat.get(&k.0).cloned(),
                    v.look.get(&k.0).cloned(),
                    v.recs.get(&k.0).map(|r| r.n),
                    v.recs.at(&k.0).items.len(),
                );
                testing::assert_view_clean();
                drop(v);
                let (want, pred) = m.view(k.0);
                assert_eq!(got, want, "view result differs\ntrace: {trace:?}");
                let r: Receipt = testing::receipt();
                compare("view", &r.ops, pred, &trace);
                logs.push(CallLog { kind: CallKind::View, ops: r.ops });
                root = begin(&mut m);
            }
            Op::RecRetyped(k, x) => {
                end(&mut m, root, &mut seg_start, &mut logs, &trace);
                if m.would_panic(op) {
                    root = begin(&mut m);
                    continue;
                }
                testing::begin_call();
                let r2 = load_root::<MRoot>();
                let got = {
                    let mut map = recs_v2();
                    Ret::OptU64(map.get_mut(&k.0).map(|r| {
                        r.note = r.note.wrapping_add(x as u32);
                        r.note as u64
                    }))
                };
                flush_root(&r2);
                drop(r2);
                let (want, pred) = m.retyped(k.0, x);
                assert_eq!(got, want, "re-typed get_mut differs\ntrace: {trace:?}");
                let r = testing::receipt();
                compare("re-typed call", &r.ops, pred, &trace);
                assert_eq!(testing::store(), m.cache.host, "storage differs after the re-typed call\ntrace: {trace:?}");
                logs.push(CallLog { kind: CallKind::Mutating, ops: r.ops });
                root = begin(&mut m);
            }
            _ => {
                if m.would_panic(op) {
                    trace.pop();
                    continue;
                }
                let want = m.apply(op);
                let got = real_apply(&mut root, op);
                assert_eq!(got, want, "return value of {op:?} differs\ntrace: {trace:?}");
            }
        }
    }
    end(&mut m, root, &mut seg_start, &mut logs, &trace);
    (logs, m.committed)
}

pub fn ops_gen() -> impl bolero::ValueGenerator<Output = std::vec::Vec<Op>> {
    bolero::produce::<std::vec::Vec<Op>>().with().len(0usize..=60)
}

/// Part A: the model is the oracle. Random engine, 2,000 sequences of up to 60 operations
/// (override with BOLERO_RANDOM_ITERATIONS).
#[test]
fn differential_against_reference_model() {
    bolero::check!().with_generator(ops_gen()).with_iterations(2000).for_each(|ops| {
        run(ops);
    });
}

/// Fixed regression sequences that exercise every operation at least once.
#[test]
fn differential_fixed_sequences() {
    use crate::state::model::{K, Val};
    let v = |len, fill| Val { len, fill };
    run(&[
        Op::FlatInsert(K(1), v(3, 1)),
        Op::FlatInsert(K(2), v(0, 0)),
        Op::FlatInsert(K(3), v(5, 2)),
        Op::EndCall,
        Op::FlatRemove(K(1)),
        Op::FlatPage(Some(K(3)), 2),
        Op::FlatPage(Some(K(1)), 2),
        Op::FlatKeysPage(None, 3),
        Op::FlatIter(1, 3),
        Op::RecTryInsert(K(0), 2),
        Op::RecAtMutInsert(K(0), K(1), v(2, 1)),
        Op::RecAtMutInsert(K(0), K(2), v(2, 1)),
        Op::RecAtMutInsert(K(0), K(3), v(2, 1)),
        Op::EndCall,
        Op::RecRemoveSome(K(0), 1),
        Op::EndCall,
        Op::RecGet(K(0)),
        Op::RecRemoveSome(K(0), 1),
        Op::EndCall,
        Op::RecRetyped(K(0), 1),
        Op::RecRemoveSome(K(0), 5),
        Op::RecEntryDefault(K(4)),
        Op::RecRetyped(K(4), 0),
        Op::RecGetMut(K(4), 1),
        Op::View(K(4)),
        Op::VecPush(v(1, 1)),
        Op::VecPush(v(1, 2)),
        Op::VecSwapRemove(0),
        Op::MidFlush,
        Op::VecPop,
        Op::VecPop,
        Op::LookInsert(K(1), v(1, 1)),
        Op::LookDiscard(K(2)),
        Op::LookEntry(K(1), v(2, 2)),
        Op::LookGetMut(K(1), Some(9)),
        Op::LookRemove(K(1)),
        Op::Counter(2),
        Op::RecRemove(K(4)),
        Op::RecPage(None, 3),
    ]);
}
