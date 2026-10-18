//! Tests closing cargo-mutants gaps in `cache.rs` (verification round 2). Each test names the
//! mutants it kills. Where no public collection call reaches the cache state a mutant changes,
//! the test drives the crate-internal `cache` API directly and checks its return value or the
//! host ops the flush issues.

use super::cache;
use super::key::{DEFAULT_ROOT, Path, kind};
use super::root::__private as p;
use super::testing::{self, end_call, seed, store};
use super::tests::Account;
use super::value::BorshLeaf;
use super::*;
use core::cell::Cell;
use std::vec::Vec as StdVec;

fn at<C: Default + Value>(tag: u32) -> C {
    let mut c = C::default();
    c.bind(&Path::root(DEFAULT_ROOT).field(tag));
    c
}

/// The entry path of `k` in a map at root field 3.
fn entry_path(k: u32) -> Path {
    Path::root(DEFAULT_ROOT).field(3).kind(kind::ENTRIES).entry(&k)
}

fn account(nonce: u64, path: &Path) -> Account {
    let mut a = Account { nonce, ..Default::default() };
    Value::bind(&mut a, path);
    a
}

// ---------------------------------------------------------------------------------------
// 1. Cost accounting: `delete_cost`, `remove_count_as`
// ---------------------------------------------------------------------------------------

/// No slot yet: a blind delete of a map entry that owns storage removes its record key and
/// its flag key (2), anything else 1. Kills `delete_cost`: `> -> <` (line 642, `flaggable`
/// never set) and `+ -> -` (line 644).
#[test]
fn gaps_cache_delete_cost_of_an_uncached_flaggable_entry_is_two() {
    testing::use_mock();
    let p = entry_path(1);
    assert_eq!(cache::delete_cost::<Account>(&p, true), 2);
    assert_eq!(cache::delete_cost::<Account>(&p, false), 1);
    assert_eq!(cache::delete_cost::<u64>(&p, true), 1);
    cache::delete_entry::<Account>(&p);
    assert_eq!(end_call().rwd(), (0, 0, 2), "the flush agrees: record key and flag key");
}

/// A slot made by a blind `put` (not flaggable, `orig` unknown): deleting it as a map entry
/// marks it flaggable and blind, so the flush removes both keys, 2 more removes than now.
/// Kills `delete_cost`: `== -> !=` (line 649) and `+= -> -=` / `+= -> *=` (line 650).
#[test]
fn gaps_cache_delete_cost_after_blind_put_counts_the_flag_key() {
    testing::use_mock();
    let p = entry_path(1);
    cache::put(&p, account(1, &p));
    assert_eq!(cache::pending_removes(), 0);
    assert_eq!(cache::delete_cost::<Account>(&p, true), 2);
    cache::delete_entry::<Account>(&p);
    assert_eq!(cache::pending_removes(), 2);
    assert_eq!(end_call().rwd(), (0, 0, 2));
}

/// A blind delete then a reinsert of a flaggable entry (review F1): the flush writes the
/// record key and removes the flag key, which may still hold the old record. So 1 pending
/// remove now, and deleting again costs 1 more (2 in all). Kills `remove_count_as`
/// `== -> !=` (line 243): the F1 branch never taken counts 0 removes now (and `pending_removes`
/// then disagrees with `flush_ops`), and `delete_cost` says 2.
#[test]
fn gaps_cache_remove_count_after_blind_delete_then_put() {
    testing::use_mock();
    let p = entry_path(1);
    cache::delete_entry::<Account>(&p);
    cache::put(&p, account(3, &p));
    assert_eq!(cache::pending_removes(), 1);
    assert_eq!(cache::delete_cost::<Account>(&p, true), 1);
    let r = end_call();
    assert_eq!(r.rwd(), (0, 1, 1));
    assert_eq!(r.ops[1].key, p.flag().as_bytes().to_vec(), "the remove is of the flag key");
}

/// A record found at its flag key (being cleared, `Clearing::FromOrig`) and handed out as
/// `&mut`: still clearing, so the flush leaves its record key alone (0 removes) and deleting
/// it costs 1 (its flag key). Kills `remove_count_as` `!= -> ==` (line 249): not clearing, it
/// counts the flag-key remove as pending (`pending_removes` disagrees with `flush_ops`) and
/// `delete_cost` says 0.
#[test]
fn gaps_cache_remove_count_of_a_touched_record_read_at_its_flag_key() {
    testing::use_mock();
    let p = entry_path(1);
    seed(p.flag().as_bytes(), &Account { nonce: 4, ..Default::default() }.encode_value(None));
    assert!(cache::get_entry::<Account>(&p).is_some());
    assert!(cache::is_clearing(&p));
    cache::touch(&p);
    assert_eq!(cache::pending_removes(), 0);
    assert_eq!(cache::delete_cost::<Account>(&p, true), 1);
    assert_eq!(end_call().rwd(), (2, 0, 0), "record-key miss, flag-key hit; bytes unchanged");
}

// ---------------------------------------------------------------------------------------
// 2. `stored_bytes`: the removal guard after a whole replace or a delete in this call
// ---------------------------------------------------------------------------------------

/// Hand-expanded scalar-only record `SlimRec { #[key(1)] nonce: u64 }` with the generated
/// unknown-collection check (`__RECORD`), so `discard` runs the API round 5 removal guard.
#[derive(Default, Debug)]
struct SlimRec {
    nonce: u64,
}
impl Leaf for SlimRec where for<'x> u64: Leaf {}
impl Removable for SlimRec {}
impl Value for SlimRec {
    const __RECORD: bool = true;
    fn __unknown_collection(bytes: &[u8]) -> Option<(StdVec<u32>, u8)> {
        let f = p::parse(bytes).ok()?;
        for &(t, w, b) in f.iter() {
            let hit = match t {
                1 => p::declared_field_collection::<u64>(w, b),
                _ => p::unknown_field_collection(w, b),
            };
            if let Some((mut trail, k)) = hit {
                trail.insert(0, t);
                return Some((trail, k));
            }
        }
        None
    }
    fn encode_value(&self, orig: Option<&[u8]>) -> StdVec<u8> {
        let of = p::parse_orig(orig);
        let mut w = p::RecordWriter::new();
        p::encode_field(&mut w, of.as_ref(), 1, &self.nonce);
        w.finish(orig, |t| t != 1)
    }
    fn decode_value(bytes: &[u8], path: &Path) -> Result<Self, RecordError> {
        let f = p::parse(bytes)?;
        Ok(Self { nonce: p::field(&f, 1, Some(path))? })
    }
}

/// `SlimRec` bytes from a newer version that also declared a `Map` at tag 9.
fn slim_with_unknown_map() -> StdVec<u8> {
    let mut w = p::RecordWriter::new();
    w.push(1, borsh::to_vec(&1u64).unwrap());
    w.push_marker(9, record::marker::MAP);
    w.finish(None, |_| true)
}

fn slim_key() -> Path {
    Path::root(DEFAULT_ROOT).field(7).kind(kind::ENTRIES).entry(&1u32)
}

/// Fixture check: the stored bytes do trip the guard.
#[test]
#[should_panic(expected = "refusing to remove the record")]
fn gaps_cache_unknown_map_tag_trips_the_removal_guard() {
    testing::use_mock();
    seed(slim_key().as_bytes(), &slim_with_unknown_map());
    let mut m: LookupMap<u32, SlimRec> = at(7);
    m.discard(&1);
}

/// After a read and a blind `insert` (a whole replace), `discard` checks nothing: the old
/// bytes are not what it removes. Kills `stored_bytes` guard `!s.merge -> false` and the
/// deleted `!` (line 630): either returns the old bytes and the guard panics.
#[test]
fn gaps_cache_discard_after_whole_replace_skips_the_old_bytes() {
    testing::use_mock();
    seed(slim_key().as_bytes(), &slim_with_unknown_map());
    let mut m: LookupMap<u32, SlimRec> = at(7);
    assert_eq!(m.get(&1).map(|s| s.nonce), Some(1));
    m.insert(1, SlimRec { nonce: 2 });
    m.discard(&1);
    assert_eq!(end_call().rwd(), (1, 0, 1));
    assert!(!store().contains_key(slim_key().as_bytes()));
}

/// A second `discard` of a key already deleted in this call checks nothing either. Kills the
/// deleted `Mode::Deleted` arm of `stored_bytes` (line 629): the deleted slot falls through to
/// its original bytes and the guard panics.
#[test]
fn gaps_cache_second_discard_after_delete_skips_the_old_bytes() {
    testing::use_mock();
    seed(slim_key().as_bytes(), &slim_with_unknown_map());
    let mut m: LookupMap<u32, SlimRec> = at(7);
    assert_eq!(m.get(&1).map(|s| s.nonce), Some(1));
    m.insert(1, SlimRec { nonce: 2 });
    m.discard(&1);
    m.discard(&1);
    assert_eq!(end_call().rwd(), (1, 0, 1));
}

// ---------------------------------------------------------------------------------------
// 3. `take_entry` / `delete_entry` / `delete_impl` / `bury`
// ---------------------------------------------------------------------------------------

/// `take_entry` and `delete_entry` of an unread map entry that owns storage are blind: the
/// flush removes its record key and its flag key. Kills `> -> <` in `take_entry` (line 741)
/// and in `delete_entry` (line 757): not flaggable, the flag key is left behind.
#[test]
fn gaps_cache_blind_take_and_delete_entry_remove_the_flag_key() {
    testing::use_mock();
    let (p1, p2) = (entry_path(1), entry_path(2));
    for p in [&p1, &p2] {
        seed(p.flag().as_bytes(), &Account::default().encode_value(None));
    }
    testing::begin_call();
    assert!(cache::take_entry::<Account>(&p1).is_none());
    cache::delete_entry::<Account>(&p2);
    let r = end_call();
    assert_eq!(r.rwd(), (0, 0, 4));
    let removed: StdVec<_> = r.ops.iter().map(|o| o.key.clone()).collect();
    for p in [&p1, &p2] {
        assert!(removed.contains(&p.as_bytes().to_vec()));
        assert!(removed.contains(&p.flag().as_bytes().to_vec()));
    }
    assert!(store().is_empty(), "no half-cleared record left at a flag key");
}

/// Only removals of values that own storage bump the removal epoch (audit A2); a leaf removal
/// leaves every handle on `check_live`'s one-compare fast path and logs nothing. Kills
/// `> -> >=` in `delete_impl` (line 770).
#[test]
fn gaps_cache_leaf_removal_does_not_bump_the_epoch() {
    testing::use_mock();
    let e = cache::epoch();
    cache::delete::<u64>(&entry_path(1));
    cache::delete_entry::<u64>(&entry_path(2));
    let _ = cache::take::<u64>(&entry_path(3));
    let _ = cache::take_entry::<u64>(&entry_path(4));
    assert_eq!(cache::epoch(), e, "leaf removals");
    cache::delete::<Account>(&entry_path(5));
    assert_eq!(cache::epoch(), e + 1, "a record that owns storage");
    end_call();
}

std::thread_local! {
    static DROPS: Cell<usize> = const { Cell::new(0) };
}
fn drops() -> usize {
    DROPS.with(Cell::get)
}

/// A leaf whose `Drop` is counted, standing in for a value whose drop is observable (a v1
/// collection's write-on-drop, invariant 4).
#[derive(borsh::BorshSerialize, borsh::BorshDeserialize)]
struct Dropper(u8);
impl BorshLeaf for Dropper {}
impl Drop for Dropper {
    fn drop(&mut self) {
        DROPS.with(|d| d.set(d.get() + 1));
    }
}

/// The flush drops every slot's value: the one written, then the one decoded by a read. Kills
/// `Slot::drop_value -> ()` and `free_slots -> ()`.
#[test]
fn gaps_cache_flush_drops_slot_values() {
    testing::use_mock();
    let mut m: LookupMap<u32, Dropper> = at(1);
    let before = drops();
    m.insert(1, Dropper(1));
    assert_eq!(drops(), before, "held by the cache until the flush");
    end_call();
    assert_eq!(drops(), before + 1, "written value dropped at the flush");
    assert_eq!(m.get(&1).map(|d| d.0), Some(1));
    assert_eq!(drops(), before + 1);
    end_call();
    assert_eq!(drops(), before + 2, "decoded value dropped at the flush");
}

/// A deleted value is buried (an escaped handle may still hold a `&V`, invariant 2) and
/// dropped at the flush, not leaked. Kills `bury -> ()` (and `free_slots -> ()`).
#[test]
fn gaps_cache_buried_value_is_dropped_at_the_flush() {
    testing::use_mock();
    let mut m: LookupMap<u32, Dropper> = at(1);
    let mut l: Lazy<Dropper> = at(2);
    m.insert(1, Dropper(1));
    l.set(Dropper(2));
    end_call();
    let before = drops();
    assert_eq!(m.get(&1).map(|d| d.0), Some(1));
    assert_eq!(l.get().map(|d| d.0), Some(2));
    m.discard(&1); // `delete_entry`
    l.remove(); // `delete`
    assert_eq!(drops(), before, "buried, not freed, until the flush");
    assert_eq!(end_call().rwd(), (2, 0, 2), "the two reads above, then the two removes");
    assert_eq!(drops(), before + 2, "both buried values dropped at the flush");
}

// ---------------------------------------------------------------------------------------
// 4. `check_live`: a fresh handle, and a handle at the removed key itself
// ---------------------------------------------------------------------------------------

/// A record removed and re-inserted at the same key: the new record's handles are stamped
/// with that removal's epoch, so a later unrelated removal makes them scan only newer
/// removals and they stay writable. Kills `> -> >=` in `check_live`'s `take_while` (line 446):
/// it rescans the removal the handle was made at and panics.
#[test]
fn gaps_cache_reinserted_record_survives_a_later_unrelated_removal() {
    testing::use_mock();
    let mut m: LookupMap<u32, Account> = at(6);
    let _ = m.try_insert(1, Account::default());
    let _ = m.try_insert(2, Account::default());
    end_call();
    assert!(m.remove(&1).is_some());
    let _ = m.try_insert(1, Account { nonce: 7, ..Default::default() }); // bound at the removal's epoch
    assert!(m.remove(&2).is_some());
    let _ = m.get_mut(&1).unwrap().balances.insert("usdt".into(), 5);
    end_call();
    assert_eq!(m.at(&1).balances.get(&"usdt".into()), Some(&5));
    assert!(m.get(&2).is_none());
}

/// A `Lazy` holding a record that owns storage: removing the value logs the `Lazy`'s own path
/// as gone, and the `Lazy` handle (at that path, not strictly below it) may set a new value.
/// Kills `> -> >=` in `check_live`'s length check (line 447): it treats the handle as stale.
#[test]
fn gaps_cache_lazy_record_set_after_remove() {
    testing::use_mock();
    let mut l: Lazy<Account> = at(5);
    l.set(Account { nonce: 1, ..Default::default() });
    end_call();
    l.remove();
    l.set(Account { nonce: 2, ..Default::default() });
    end_call();
    assert_eq!(l.get().map(|a| a.nonce), Some(2));
}
