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
    if op.is_nested() {
        return real_apply_nested(root, op);
    }
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
        Op::RecRemoveSome(k, n) => Ret::Bool(
            root.recs.remove_step(&k.0, core::num::NonZeroU32::new(n as u32).expect("generator 3..13"))
                == crate::state::Removal::Pending,
        ),
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
        _ => unreachable!("nested ops are dispatched to real_apply_nested"),
    }
}

/// The Verification round 2 ops: records owning a `Lazy`, a `Set` and a `Vector` (`xrecs`), and
/// records owning a `Set` (`lrecs`; a `LookupSet` until round 7). Writes go through `at_mut`, reads through `at`.
fn real_apply_nested(root: &mut MRoot, op: &Op) -> Ret {
    use super::fixture::{RecL, RecX};
    let push = |v: &mut Vec<u8>, x: Option<u8>| {
        if let Some(b) = x {
            v.push(b);
        }
        v.clone()
    };
    match *op {
        Op::XTryInsert(k, n) => Ret::Bool(root.xrecs.try_insert(k.0, RecX { n: n as u64, ..Default::default() }).is_ok()),
        Op::XEntryDefault(k) => Ret::U64(root.xrecs.entry(k.0).or_default().n),
        Op::XGet(k) => Ret::OptU64(root.xrecs.get(&k.0).map(|r| r.n)),
        Op::XGetMut(k, x) => Ret::OptU64(root.xrecs.get_mut(&k.0).map(|r| {
            r.n = r.n.wrapping_add(x as u64);
            r.n
        })),
        Op::XRemove(k) => Ret::OptU64(root.xrecs.remove(&k.0).map(|r| r.n)),
        Op::XDiscard(k) => Ret::Bool(root.xrecs.discard(&k.0)),
        Op::XRemoveStep(k, n) => Ret::Bool(
            root.xrecs.remove_step(&k.0, core::num::NonZeroU32::new(n as u32).expect("generator 3..13"))
                == crate::state::Removal::Pending,
        ),
        Op::XLazySet(k, v) => match root.xrecs.at_mut(&k.0) {
            None => Ret::OptU64(None),
            Some(r) => {
                r.lazy.set(v.bytes());
                Ret::Bool(true)
            }
        },
        Op::XLazyGet(k) => Ret::Bytes(root.xrecs.at(&k.0).lazy.get().cloned()),
        Op::XLazyGetMut(k, x) => match root.xrecs.at_mut(&k.0) {
            None => Ret::OptU64(None),
            Some(r) => Ret::Bytes(r.lazy.get_mut().map(|v| push(v, x))),
        },
        Op::XLazyRemove(k) => match root.xrecs.at_mut(&k.0) {
            None => Ret::OptU64(None),
            Some(r) => {
                r.lazy.remove();
                Ret::Bool(true)
            }
        },
        Op::XSetInsert(k, x) => match root.xrecs.at_mut(&k.0) {
            None => Ret::OptU64(None),
            Some(r) => Ret::Bool(r.set.insert(x.0)),
        },
        Op::XSetRemove(k, x) => match root.xrecs.at_mut(&k.0) {
            None => Ret::OptU64(None),
            Some(r) => Ret::Bool(r.set.remove(&x.0)),
        },
        Op::XSetContains(k, x) => Ret::Bool(root.xrecs.at(&k.0).set.contains(&x.0)),
        Op::XSetLen(k) => Ret::U32(root.xrecs.at(&k.0).set.len()),
        Op::XSetIter(k, skip, take) => Ret::Items(
            root.xrecs.at(&k.0).set.iter().skip(skip as usize).take(take as usize).map(|x| (x, Vec::new())).collect(),
        ),
        Op::XVecPush(k, v) => match root.xrecs.at_mut(&k.0) {
            None => Ret::OptU64(None),
            Some(r) => {
                r.vect.push(v.bytes());
                Ret::Bool(true)
            }
        },
        Op::XVecPop(k) => match root.xrecs.at_mut(&k.0) {
            None => Ret::OptU64(None),
            Some(r) => Ret::Bytes(r.vect.pop()),
        },
        Op::XVecSwapRemove(k, i) => match root.xrecs.at_mut(&k.0) {
            None => Ret::OptU64(None),
            Some(r) => Ret::Bytes(Some(r.vect.swap_remove(i as u32))),
        },
        Op::XVecGet(k, i) => Ret::Bytes(root.xrecs.at(&k.0).vect.get(i as u32).cloned()),
        Op::XVecGetMut(k, i, x) => match root.xrecs.at_mut(&k.0) {
            None => Ret::OptU64(None),
            Some(r) => Ret::Bytes(r.vect.get_mut(i as u32).map(|v| push(v, x))),
        },
        Op::XVecLen(k) => Ret::U32(root.xrecs.at(&k.0).vect.len()),
        Op::XVecIterBack(k, skip) => {
            let it = root.xrecs.at(&k.0).vect.iter().rev().skip(skip as usize);
            let (lo, hi) = it.size_hint();
            assert_eq!(hi, Some(lo), "IndexIter size_hint is exact");
            Ret::Iter(it.take(2).cloned().collect(), lo as u32)
        }
        Op::LTryInsert(k, n) => Ret::Bool(root.lrecs.try_insert(k.0, RecL { n: n as u64, ..Default::default() }).is_ok()),
        Op::LGet(k) => Ret::OptU64(root.lrecs.get(&k.0).map(|r| r.n)),
        Op::LGetMut(k, x) => Ret::OptU64(root.lrecs.get_mut(&k.0).map(|r| {
            r.n = r.n.wrapping_add(x as u64);
            r.n
        })),
        Op::LSetInsert(k, x) => match root.lrecs.at_mut(&k.0) {
            None => Ret::OptU64(None),
            Some(r) => Ret::Bool(r.lset.insert(x.0)),
        },
        Op::LSetRemove(k, x) => match root.lrecs.at_mut(&k.0) {
            None => Ret::OptU64(None),
            Some(r) => Ret::Bool(r.lset.remove(&x.0)),
        },
        Op::LSetContains(k, x) => Ret::Bool(root.lrecs.at(&k.0).lset.contains(&x.0)),
        _ => unreachable!("not a nested op: {op:?}"),
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
        Op::RecRemoveSome(K(0), 4),
        Op::EndCall,
        Op::RecGet(K(0)),
        Op::RecRemoveSome(K(0), 4),
        Op::EndCall,
        Op::RecRetyped(K(0), 1),
        Op::RecRemoveSome(K(0), 9),
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

/// Verification round 2: fixed sequences over records owning a `Lazy`, a `Set` and a `Vector`
/// (`xrecs`) and a `Set` under a `LookupMap` (`lrecs`), every nested op at least once, including a
/// `remove_step` that takes several calls (Lazy, Set entries, Vector slots, headers) and one
/// interrupted by writes from the same call.
#[test]
fn differential_fixed_nested_sequences() {
    use crate::state::model::{K, Val};
    let v = |len, fill| Val { len, fill };
    let fill = |k: u8| {
        let mut ops = vec![
            Op::XTryInsert(K(k), 1),
            Op::XLazySet(K(k), v(3, 1)),
            Op::XSetInsert(K(k), K(1)),
            Op::XSetInsert(K(k), K(2)),
            Op::XSetInsert(K(k), K(3)),
            Op::XVecPush(K(k), v(1, 1)),
            Op::XVecPush(K(k), v(2, 2)),
            Op::XVecPush(K(k), v(1, 1)),
        ];
        ops.push(Op::EndCall);
        ops
    };
    let mut ops = fill(0);
    ops.extend([
        Op::XGet(K(0)),
        Op::XLazyGet(K(0)),
        Op::XLazyGetMut(K(0), Some(7)),
        Op::XSetContains(K(0), K(2)),
        Op::XSetContains(K(0), K(4)),
        Op::XSetLen(K(0)),
        Op::XSetIter(K(0), 1, 3),
        Op::XVecLen(K(0)),
        Op::XVecGet(K(0), 1),
        Op::XVecIterBack(K(0), 0),
        Op::XVecIterBack(K(0), 2),
        Op::XVecIterBack(K(0), 3),
        Op::EndCall,
        Op::XSetRemove(K(0), K(1)),
        Op::XSetRemove(K(0), K(4)),
        Op::XVecSwapRemove(K(0), 0),
        Op::XVecGetMut(K(0), 0, Some(9)),
        Op::XVecPop(K(0)),
        Op::XGetMut(K(0), 2),
        Op::EndCall,
        // budgets 3, then 4: several calls, every unit kind
        Op::XRemoveStep(K(0), 3),
        Op::EndCall,
        Op::XGet(K(0)),
        Op::XTryInsert(K(0), 2),
        Op::XRemoveStep(K(0), 3),
        Op::EndCall,
        Op::XRemoveStep(K(0), 4),
        Op::EndCall,
        Op::XRemoveStep(K(0), 4),
        Op::EndCall,
        Op::XRemoveStep(K(0), 12),
        Op::EndCall,
    ]);
    ops.extend(fill(1));
    ops.extend([Op::XLazyRemove(K(1)), Op::XLazyRemove(K(1)), Op::XRemove(K(1)), Op::EndCall]);
    ops.extend(fill(2));
    ops.extend([Op::XRemoveStep(K(2), 5), Op::EndCall, Op::XDiscard(K(2)), Op::XDiscard(K(3)), Op::EndCall]);
    ops.extend(fill(3));
    ops.extend([Op::XEntryDefault(K(3)), Op::XEntryDefault(K(4)), Op::XRemoveStep(K(3), 3), Op::MidFlush, Op::XRemoveStep(K(3), 6), Op::EndCall, Op::XRemoveStep(K(3), 12), Op::EndCall]);
    ops.extend([
        Op::LTryInsert(K(0), 1),
        Op::LSetInsert(K(0), K(1)),
        Op::LSetInsert(K(1), K(1)),
        Op::LSetContains(K(0), K(1)),
        Op::EndCall,
        Op::LTryInsert(K(0), 2),
        Op::LGet(K(0)),
        Op::LGetMut(K(0), 2),
        Op::LSetContains(K(0), K(1)),
        Op::LSetRemove(K(0), K(1)),
        Op::LSetRemove(K(0), K(2)),
        Op::LSetContains(K(0), K(1)),
        Op::LGet(K(3)),
    ]);
    let (logs, st) = run_state(&ops);
    assert!(st.xrecs.iter().all(|(k, _)| *k == 4), "only the defaulted record 4 is left: {:?}", st.xrecs);
    assert_eq!(st.lrecs.len(), 1);
    // the multi-call clear of record 0 removed at most n keys per call
    assert!(logs.iter().filter(|l| l.kind == CallKind::Mutating).count() > 10);
}

/// Verification round 2: how often the random op generator reaches the nested paths (a
/// regression guard on the generator, not a property). Over 200 histories, every nested op kind
/// must run at least once against a present record.
#[test]
fn differential_generator_reaches_nested_ops() {
    use std::collections::BTreeSet;
    let mut seen = BTreeSet::new();
    bolero::check!().with_generator(ops_gen()).with_iterations(200).for_each(|ops| {
        for o in ops.iter().filter(|o| o.is_nested()) {
            seen.insert(format!("{o:?}").split('(').next().unwrap().to_string());
        }
        run(ops);
    });
    assert!(seen.len() >= 25, "nested op kinds generated: {seen:?}");
}

/// Verification round 2, Part A restricted to the nested ops: histories of up to 60 ops drawn
/// only from the `xrecs` / `lrecs` ops plus call boundaries (`EndCall`, `MidFlush`), so a
/// record grows several `Lazy` / `Set` / `Vector` children and is then removed or cleared in
/// steps. Random engine, 2,000 histories. Same oracle and comparisons as
/// `differential_against_reference_model`.
#[test]
fn differential_nested_against_reference_model() {
    let g = bolero::produce::<std::vec::Vec<Op>>().with().len(0usize..=240);
    bolero::check!().with_generator(g).with_iterations(2000).for_each(|ops| {
        let ops: std::vec::Vec<Op> =
            ops.iter().filter(|o| o.is_nested() || matches!(o, Op::EndCall | Op::MidFlush)).take(60).cloned().collect();
        run(&ops);
    });
}
