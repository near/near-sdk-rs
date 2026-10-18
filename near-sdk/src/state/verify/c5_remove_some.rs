//! Bug class C5, bounded-clearing isolation (`remove_step`, `remove_some` before API round 3).
//! While a record is being cleared in bounded steps: every write below it (`get_mut`, `entry`,
//! `at_mut`, an insert through `at_mut`, and, since API round 4, a write through a handle to
//! its map made before `remove_step` flagged it: the generation check) panics, in the same
//! call and in every later one, and changes no stored byte; `get` and `at` still read it; each
//! call removes exactly `min(n, remaining)` entries, so clearing cannot be outrun and finishes
//! in `ceil(m / n) + 1` calls at most; the flag is written only by the FIRST unfinished call
//! (since API round 2 the flag is the record moving to its flag key `record_key ++ 00`: that
//! call removes the record key, then writes the flag key, in key order); the finishing call
//! removes the record (from its flag key) and leaves nothing under it; a neighbour record is
//! untouched. Real panics, caught with `catch_unwind` (random engine only: Kani cannot compile
//! `catch_unwind`, see `g4_one_write`).

use super::c2_orphans::assert_no_orphans;
use super::fixture::{MRoot, Rec};
use crate::state::model;
use crate::state::testing::{self, OpKind};
use crate::state::{flush_root, load_root};
use bolero::generator::TypeGenerator;
use std::panic::{AssertUnwindSafe, catch_unwind};

#[derive(Clone, Copy, Debug, TypeGenerator)]
pub enum Attempt {
    GetMut,
    EntryOrDefault,
    AtMut,
    AtMutInsert(#[generator(0..12)] u8),
    AtMutRemove(#[generator(0..12)] u8),
    /// API round 4: a handle to record 3's items map made BEFORE `remove_step` in the same
    /// call (an escaped handle, as `mem::take` on the field gives), written through after it.
    /// In a later call (no `remove_step` before the write) it is an `AtMutInsert`: the
    /// generation check is per call, and a handle made in a call with no removal is fresh.
    Escaped(#[generator(0..12)] u8),
}

fn items3() -> crate::state::Map<u8, std::vec::Vec<u8>> {
    use super::fixture::{REC_ITEMS, TAG_RECS};
    use crate::state::key::{DEFAULT_ROOT, Path, kind};
    crate::state::Map::__bound_at(Path::root(DEFAULT_ROOT).field(TAG_RECS).kind(kind::ENTRIES).entry(&3u8).field(REC_ITEMS))
}

fn attempt(root: &mut MRoot, a: Attempt) {
    match a {
        Attempt::Escaped(j) => {
            root.recs.at_mut(&3).unwrap().items.insert(j, vec![0xEE]);
        }
        Attempt::GetMut => root.recs.get_mut(&3).unwrap().n += 1,
        Attempt::EntryOrDefault => root.recs.entry(3).or_default().n += 1,
        Attempt::AtMut => drop(root.recs.at_mut(&3)),
        Attempt::AtMutInsert(j) => {
            root.recs.at_mut(&3).unwrap().items.insert(j, vec![0xEE]);
        }
        Attempt::AtMutRemove(j) => {
            root.recs.at_mut(&3).unwrap().items.remove(&j);
        }
    }
}

fn quiet<R>(f: impl FnOnce() -> R) -> std::thread::Result<R> {
    let hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));
    let r = catch_unwind(AssertUnwindSafe(f));
    std::panic::set_hook(hook);
    r
}

/// Random engine, 3,000 cases: target record 3 with 1..=8 items and a neighbour record 0
/// with 2 items; up to 12 clearing calls with budgets 0..=3, each followed (and, in the same
/// call, preceded by nothing else) by up to 2 write attempts in their own calls plus 1 in the
/// clearing call itself after `remove_some` returned true.
#[test]
fn c5_remove_some_isolates_the_record_until_it_is_gone() {
    let g = (1usize..=8, bolero::produce::<[(u8, Option<Attempt>, Option<Attempt>); 12]>());
    bolero::check!().with_generator(g).with_iterations(3_000).for_each(|(m, steps)| {
        testing::use_mock();
        let mut root = load_root::<MRoot>();
        root.recs.try_insert(0, Rec::default()).unwrap();
        root.recs.at_mut(&0).unwrap().items.insert(0, vec![1]);
        root.recs.at_mut(&0).unwrap().items.insert(1, vec![2]);
        root.recs.try_insert(3, Rec { n: 9, ..Default::default() }).unwrap();
        for j in 0..*m {
            root.recs.at_mut(&3).unwrap().items.insert(j as u8, vec![j as u8]);
        }
        flush_root(&root);
        drop(root);
        let rec_key = model::recs().entry(3);
        let flag_key = model::rec_flag(3);
        let mut remaining = *m as u32;
        let mut unfinished_calls = 0;
        let mut finished = false;
        let mut positive_calls = 0u32;
        let mut min_n = u32::MAX;
        for (n, a1, a2) in steps.iter() {
            let n = (*n % 4) as u32;
            testing::begin_call();
            let mut root = load_root::<MRoot>();
            let mut escaped = matches!(a1, Some(Attempt::Escaped(_))).then(items3);
            let again = root.recs.remove_step(&3, n) == crate::state::Removal::Pending;
            let removed = n.min(remaining);
            if n > 0 {
                positive_calls += 1;
                min_n = min_n.min(n);
            }
            if again {
                assert_eq!(root.recs.at(&3).items.len(), remaining - removed, "progress is exactly min(n, remaining)");
                assert_eq!(root.recs.get(&3).map(|r| r.n), Some(9), "get still reads a clearing record");
            }
            // a write in the same call, after remove_some said "call again"
            let mid = again.then_some(a1.unwrap_or(Attempt::AtMut));
            if let Some(a) = mid {
                let before = testing::store();
                let r = match (a, escaped.as_mut()) {
                    (Attempt::Escaped(j), Some(h)) => quiet(|| {
                        h.insert(j, vec![0xEE]);
                    }),
                    _ => quiet(|| attempt(&mut root, a)),
                };
                assert!(r.is_err(), "{a:?} in the clearing call did not panic");
                drop(escaped);
                drop(root);
                testing::begin_call();
                assert_eq!(testing::store(), before);
                // the panicking call is aborted; redo the clearing call alone
                let mut root = load_root::<MRoot>();
                assert_eq!(root.recs.remove_step(&3, n) == crate::state::Removal::Pending, again);
                flush_root(&root);
            } else {
                drop(escaped);
                flush_root(&root);
                drop(root);
            }
            let rc = testing::receipt();
            let pos = |kind: OpKind, key: &[u8]| rc.ops.iter().position(|o| o.kind == kind && o.key == key);
            let count = |kind: OpKind, key: &[u8]| rc.ops.iter().filter(|o| o.kind == kind && o.key == key).count();
            let flag_writes = count(OpKind::Write, &flag_key);
            if again {
                remaining -= removed;
                unfinished_calls += 1;
                let first = unfinished_calls == 1;
                assert_eq!(flag_writes, usize::from(first), "flag written only by the first unfinished call");
                assert_eq!(count(OpKind::Remove, &rec_key), usize::from(first), "record key removed only by the first unfinished call");
                assert_eq!(count(OpKind::Write, &rec_key), 0, "a clearing call wrote the record key");
                if first {
                    assert!(pos(OpKind::Remove, &rec_key) < pos(OpKind::Write, &flag_key), "record key removed after the flag write (F4 key order)");
                }
            } else {
                let at = if unfinished_calls > 0 { &flag_key } else { &rec_key };
                assert_eq!(count(OpKind::Remove, at), 1, "finishing call did not remove the record from where it lived");
                finished = true;
                break;
            }
            // writes in later calls panic and change nothing
            for a in [a2].into_iter().flatten() {
                let a = match *a {
                    Attempt::Escaped(j) => Attempt::AtMutInsert(j),
                    a => a,
                };
                testing::begin_call();
                let before = testing::store();
                let r = quiet(|| {
                    let mut root = load_root::<MRoot>();
                    attempt(&mut root, a);
                    flush_root(&root);
                });
                assert!(r.is_err(), "{a:?} in a later call did not panic while clearing");
                testing::begin_call();
                assert_eq!(testing::store(), before, "a rejected write changed storage");
            }
        }
        if finished {
            assert!(testing::store().keys().all(|k| !k.starts_with(&rec_key)), "keys left under the cleared record");
            if min_n != u32::MAX {
                assert!(positive_calls <= (*m as u32).div_ceil(min_n) + 1, "clearing took {positive_calls} positive-budget calls for {m} items");
            }
        }
        assert_no_orphans();
        testing::begin_call();
        let root = load_root::<MRoot>();
        assert_eq!(root.recs.at(&0).items.len(), 2, "neighbour touched");
        assert_eq!(root.recs.contains_key(&3), !finished);
    });
}

/// Pinned liveness edge: `remove_step(k, 0)` on a record with entries makes no progress and
/// returns `Pending`, so a caller looping with n = 0 never finishes. The first such call moves
/// the record to its flag key (1 remove + 1 write, API round 2); every later one costs 2 reads
/// (record key miss, flag key) plus the items header and 1 slot, and writes nothing.
#[test]
fn c5_pinned_budget_zero_makes_no_progress() {
    testing::use_mock();
    let mut root = load_root::<MRoot>();
    root.recs.try_insert(3, Rec::default()).unwrap();
    root.recs.at_mut(&3).unwrap().items.insert(1, vec![1]);
    flush_root(&root);
    drop(root);
    for call in 0..3 {
        testing::begin_call();
        let mut root = load_root::<MRoot>();
        assert_eq!(root.recs.remove_step(&3, 0), crate::state::Removal::Pending);
        assert_eq!(root.recs.at(&3).items.len(), 1);
        flush_root(&root);
        drop(root);
        let rc = testing::receipt();
        assert_eq!(rc.writes() + rc.removes(), if call == 0 { 2 } else { 0 });
    }
}
