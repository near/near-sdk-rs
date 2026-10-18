//! Bug class C2, orphaned children. Removing a record (by `remove`, `discard`, or
//! `remove_some` to completion) leaves no key under its path; and at every point of every
//! history, each stored key under the record map is bookkeeping of a LIVE record, never of a
//! removed one. An invariant over the real store, read through the real API (independent of
//! the model). The v1-nested exception (a v1 collection inside a v2 record is not cleared by
//! the record's removal) is pinned in `tests/state_v2_exchange.rs`,
//! `legacy::removing_a_record_with_a_nested_v1_field_orphans_its_entries`.

use super::diff::{ops_gen, run_state};
use super::fixture::{MRoot, Rec};
use crate::state::model::{self, spec_key::MapKeys};
use crate::state::testing;
use crate::state::{flush_root, load_root};

fn field_prefix(mk: &MapKeys) -> std::vec::Vec<u8> {
    let h = mk.header();
    h[..h.len() - 1].to_vec()
}

/// Is `key` live bookkeeping of map `mk` holding `keys` (in key-vector order)?
fn map_key(mk: &MapKeys, keys: &[u8], key: &[u8]) -> bool {
    key == mk.header()
        || keys.iter().any(|k| key == mk.entry(*k) || key == mk.index(*k))
        || (0..keys.len() as u32).any(|i| key == mk.slot(i))
}

/// The orphan invariant on the current store. Returns the number of keys checked.
pub fn assert_no_orphans() -> usize {
    testing::begin_call();
    let root = load_root::<MRoot>();
    let live = root.recs.keys_page(None, 255).expect("no cursor").keys;
    let items: std::vec::Vec<(u8, std::vec::Vec<u8>)> =
        live.iter().map(|k| (*k, root.recs.at(k).items.keys_page(None, 255).expect("no cursor").keys)).collect();
    drop(root);
    testing::begin_call();
    let rk = model::recs();
    let under = field_prefix(&rk);
    let mut n = 0;
    for key in testing::store().keys().filter(|k| k.starts_with(&under)) {
        n += 1;
        if map_key(&rk, &live, key) {
            continue;
        }
        let owner = items.iter().find(|(k, _)| key.starts_with(&rk.entry(*k)));
        let Some((k, its)) = owner else {
            panic!("orphan: {key:02x?} under the record map belongs to no live record (live {live:?})");
        };
        assert!(map_key(&model::items(*k), its, key), "orphan: {key:02x?} under live record {k} is not live bookkeeping of its items {its:?}");
    }
    n
}

/// Random engine, 2,000 histories of up to 60 operations (the Part A generator: insert_new,
/// at_mut inserts and removes, remove, remove_some with budgets 0..=3, retyped rewrites,
/// mid-call flushes, spread over calls): after the history no stored key under the record map
/// is orphaned.
#[test]
fn c2_no_orphans_after_random_histories() {
    bolero::check!().with_generator(ops_gen()).with_iterations(2_000).for_each(|ops| {
        run_state(ops);
        assert_no_orphans();
    });
}

/// How a record is removed.
#[derive(Clone, Copy, Debug, bolero::generator::TypeGenerator)]
pub enum How {
    Remove,
    Discard,
    RemoveSome(#[generator(1..=3)] u8),
}

/// Random engine, 2,000 cases: neighbour records 0..=2 and target record 3 with 0..=7 items
/// (written over 1..=3 calls), the target removed by `remove`, `discard` or `remove_some(n)`
/// repeated until it returns false (one call each). Afterwards NO stored key starts with the
/// target's entry key, the neighbours' items are intact, and the invariant holds.
#[test]
fn c2_removed_record_leaves_no_key_under_its_path() {
    let g = (0usize..=7, 1u8..=3, bolero::produce::<How>(), bolero::produce::<[u8; 3]>());
    bolero::check!().with_generator(g).with_iterations(2_000).for_each(|(m, calls, how, nb)| {
        testing::use_mock();
        let mut root = load_root::<MRoot>();
        for (i, c) in nb.iter().enumerate() {
            root.recs.insert_new(i as u8, Rec::default());
            for j in 0..c % 4 {
                root.recs.at_mut(&(i as u8)).unwrap().items.insert(j, vec![i as u8, j]);
            }
        }
        root.recs.insert_new(3, Rec { n: 9, ..Default::default() });
        for j in 0..*m {
            if j % (*m / *calls as usize).max(1) == 0 {
                flush_root(&root);
                drop(root);
                testing::begin_call();
                root = load_root::<MRoot>();
            }
            root.recs.at_mut(&3).unwrap().items.insert(j as u8, vec![j as u8; j % 3]);
        }
        flush_root(&root);
        drop(root);
        let mut rounds = 0;
        loop {
            testing::begin_call();
            let mut root = load_root::<MRoot>();
            let again = match how {
                How::Remove => root.recs.remove(&3).map(|r| r.n) != Some(9),
                How::Discard => !root.recs.discard(&3),
                How::RemoveSome(n) => root.recs.remove_some(&3, *n as u32),
            };
            flush_root(&root);
            drop(root);
            rounds += 1;
            if !again {
                break;
            }
            assert!(matches!(how, How::RemoveSome(_)) && rounds <= 8, "{how:?} did not finish");
        }
        let gone = model::recs().entry(3);
        let left: std::vec::Vec<_> = testing::store().into_keys().filter(|k| k.starts_with(&gone)).collect();
        assert!(left.is_empty(), "{how:?} left {left:02x?} under the removed record");
        assert_no_orphans();
        testing::begin_call();
        let root = load_root::<MRoot>();
        for (i, c) in nb.iter().enumerate() {
            assert_eq!(root.recs.at(&(i as u8)).items.len(), (c % 4) as u32, "neighbour {i} lost items");
        }
    });
}

/// `insert_new` on an existing record panics, in a fresh call and in the same call, instead
/// of overwriting it (an overwrite would orphan the old record's children), for `Map` and
/// `LookupMap` of records. The model skips this op as a predicted panic, so nothing else
/// asserted it. (The surviving `assert_insert_new` mutant is a different path: `Lazy::set` of
/// a record over an existing one, still uncovered.)
#[test]
fn c2_insert_new_on_an_existing_record_panics() {
    let q = |f: &mut dyn FnMut()| {
        let hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(|_| {}));
        let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));
        std::panic::set_hook(hook);
        r
    };
    testing::use_mock();
    let mut root = load_root::<MRoot>();
    root.recs.insert_new(1, Rec::default());
    root.recs.at_mut(&1).unwrap().items.insert(7, vec![7]);
    assert!(q(&mut || { let _ = root.recs.insert_new(1, Rec::default()); }).is_err(), "same call: overwrote");
    drop(root);
    testing::begin_call();
    let mut root = load_root::<MRoot>();
    root.recs.insert_new(1, Rec::default());
    root.recs.at_mut(&1).unwrap().items.insert(7, vec![7]);
    flush_root(&root);
    drop(root);
    testing::begin_call();
    let before = testing::store();
    let mut root = load_root::<MRoot>();
    assert!(q(&mut || { let _ = root.recs.insert_new(1, Rec::default()); }).is_err(), "later call: overwrote");
    drop(root);
    testing::begin_call();
    assert_eq!(testing::store(), before);
    // the same through a LookupMap of records (its own guard, `assert_insert_new`)
    let path = crate::state::key::Path::root(crate::state::key::DEFAULT_ROOT).field(20);
    let mut lm: crate::state::LookupMap<u8, Rec> = crate::state::LookupMap::__bound_at(path.clone());
    lm.insert_new(1, Rec::default());
    lm.at_mut(&1).unwrap().items.insert(7, vec![7]);
    assert!(q(&mut || { let _ = lm.insert_new(1, Rec::default()); }).is_err(), "LookupMap same call: overwrote");
    drop(lm);
    testing::begin_call();
    let mut lm: crate::state::LookupMap<u8, Rec> = crate::state::LookupMap::__bound_at(path.clone());
    lm.insert_new(1, Rec::default());
    lm.at_mut(&1).unwrap().items.insert(7, vec![7]);
    testing::flush();
    drop(lm);
    testing::begin_call();
    let mut lm: crate::state::LookupMap<u8, Rec> = crate::state::LookupMap::__bound_at(path);
    assert!(q(&mut || { let _ = lm.insert_new(1, Rec::default()); }).is_err(), "LookupMap later call: overwrote");
}
