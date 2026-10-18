//! Bug class C3, lost writes under eviction and re-entry. The v2 cache never evicts within a
//! call; the eviction points are the flush (mid-call `state_write`, which frees every slot) and
//! the call boundary, and re-entry is a later call (a callback) or a second root alive in one
//! call. Checked:
//!
//! - metamorphic (independent of the model): inserting a mid-call flush at ANY position of a
//!   history leaves the final store byte-identical (no write lost or duplicated across the
//!   eviction), while the model still agrees at every step (the driver checks it). One
//!   exception since API round 5: `remove_step`'s budget counts the host removes actually
//!   issued, and a flush changes which keys the cache knows to be stored (a record inserted
//!   earlier in the call costs different removes before and after it is flushed), so a
//!   `remove_step` after the flush in the same call may progress differently and the histories
//!   then diverge legitimately. Such cases are counted, not failed; both runs still agree with
//!   the model at every step;
//! - fixed shapes: write / flush / read-back / write again in one call; `get_mut` and
//!   `at_mut` on the same record in one call; a later call (re-entry) sees every write;
//! - pinned hazard: two roots alive in one call (`state_read()` twice, not type-enforced; the
//!   `#[near]` wrapper never does it) lose the first root's scalar writes.

use super::diff::{ops_gen, run_state};
use super::fixture::{MRoot, Rec};
use crate::state::model::Op;
use crate::state::testing;
use crate::state::{flush_root, load_root};

/// Random engine, 2,000 histories of up to 60 operations with every `MidFlush` removed, then
/// one inserted at position p (any p): final store identical to the history without it.
#[test]
fn c3_a_mid_call_flush_anywhere_loses_no_write() {
    let g = (ops_gen(), bolero::produce::<u8>());
    bolero::check!().with_generator(g).with_iterations(2_000).for_each(|(ops, p)| {
        let base: std::vec::Vec<Op> = ops.iter().filter(|o| !matches!(o, Op::MidFlush)).cloned().collect();
        run_state(&base);
        let want = testing::store();
        let mut with = base.clone();
        let at = *p as usize % (base.len() + 1);
        with.insert(at, Op::MidFlush);
        run_state(&with);
        if testing::store() != want {
            // API round 5: only a budgeted clear after the flush, in the same call, may differ
            let rest = &with[at + 1..];
            let call = &rest[..rest.iter().position(|o| matches!(o, Op::EndCall)).unwrap_or(rest.len())];
            assert!(
                call.iter().any(|o| matches!(o, Op::RecRemoveSome(..))),
                "a mid-call flush changed the final state"
            );
            DIVERGED.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        }
    });
    println!(
        "c3: {} of 2,000 histories diverged through remove_step progress after the flush",
        DIVERGED.load(std::sync::atomic::Ordering::Relaxed)
    );
}

static DIVERGED: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

/// Fixed shapes, in one call: write, flush (frees every slot), read back from storage, write
/// again, flush: both writes land, the second wins, and a later call sees it; `get_mut` then
/// `at_mut` on the same record both persist; the root scalar written before and after a
/// mid-call flush ends at its last value.
#[test]
fn c3_fixed_shapes_write_flush_read_write() {
    testing::use_mock();
    let mut root = load_root::<MRoot>();
    root.flat.insert(1, vec![1]);
    root.counter = 1;
    flush_root(&root);
    assert_eq!(root.flat.get(&1), Some(&vec![1]), "read-back after the flush");
    root.flat.insert(1, vec![2]);
    root.counter = 2;
    root.recs.try_insert(0, Rec::default()).unwrap();
    flush_root(&root);
    root.recs.get_mut(&0).unwrap().n = 5;
    root.recs.at_mut(&0).unwrap().items.insert(3, vec![3]);
    flush_root(&root);
    drop(root);
    testing::begin_call();
    let root = load_root::<MRoot>();
    assert_eq!((root.flat.get(&1), root.counter), (Some(&vec![2]), 2));
    assert_eq!(root.recs.get(&0).map(|r| r.n), Some(5), "get_mut write lost");
    assert_eq!(root.recs.at(&0).items.get(&3), Some(&vec![3]), "at_mut write lost");
}

/// HAZARD, pinned: two roots alive in one call. Root A sets the counter, root B (loaded
/// before A flushed) still holds the old value and flushes last, so A's scalar write is
/// lost. Collection writes are not lost (both handles share the cache slots, which is the
/// aliasing the cache's safety invariant 3 forbids). Not reachable through the `#[near]`
/// wrapper, which loads one root per call.
#[test]
fn hazard_two_roots_in_one_call_lose_a_scalar_write() {
    testing::use_mock();
    let mut a = load_root::<MRoot>();
    let b = load_root::<MRoot>();
    a.counter = 7;
    flush_root(&a);
    flush_root(&b);
    drop(a);
    drop(b);
    testing::begin_call();
    assert_eq!(load_root::<MRoot>().counter, 0, "HAZARD: root A's counter write was lost");
}
