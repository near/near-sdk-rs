//! Unit tests for the v2 prototype against the counting mock host.
//!
//! `Account` and `Ex` below are HAND-EXPANDED: they are exactly what `#[near(storage)]` and
//! `contract_state(layout = v2)` emit (see near-sdk-macros/src/core_impl/code_generator/storage_v2.rs),
//! written out because `::near_sdk` paths do not resolve inside the crate itself.

use super::key::{DEFAULT_ROOT, Path, kind};
use super::root::__private as p;
use super::testing::{self, end_call, receipt, seed, store};
use super::*;
use crate::AccountId;
use core::marker::PhantomData;
use core::num::NonZeroU32;
use std::vec::Vec as StdVec;

/// A `remove_step` budget (API round 5: host removes, at least 3).
fn nz(n: u32) -> NonZeroU32 {
    NonZeroU32::new(n).unwrap()
}

// ---------------------------------------------------------------------------------------
// Hand-expanded #[near(storage)] struct Account { #[key(1)] nonce: u64,
//                                                #[key(2)] balances: Map<String, u128> }
// ---------------------------------------------------------------------------------------

#[derive(Default, Debug)]
pub struct Account {
    pub nonce: u64,
    pub balances: Map<String, u128>,
}

impl Value for Account {
    const MAX_SUBPATH: usize = p::max(
        p::field_subpath::<u64>(1),
        p::field_subpath::<Map<String, u128>>(2),
    );
    const __RECORD: bool = true;
    fn __unknown_collection(bytes: &[u8]) -> Option<(StdVec<u32>, u8)> {
        let f = p::parse(bytes).ok()?;
        for &(t, w, b) in f.iter() {
            let hit = match t {
                1 => p::declared_field_collection::<u64>(w, b),
                2 => p::declared_field_collection::<Map<String, u128>>(w, b),
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
        p::encode_field(&mut w, of.as_ref(), 2, &self.balances);
        w.finish(orig, |t| match t {
            1 => !<u64 as Value>::INLINE,
            2 => !<Map<String, u128> as Value>::INLINE,
            _ => true,
        })
    }
    fn decode_value(bytes: &[u8], path: &Path) -> Result<Self, RecordError> {
        let f = p::parse(bytes)?;
        Ok(Self { nonce: p::field(&f, 1, Some(path))?, balances: p::field(&f, 2, Some(path))? })
    }
    fn bind(&mut self, path: &Path) {
        Value::bind(&mut self.nonce, &path.field(1));
        Value::bind(&mut self.balances, &path.field(2));
    }
    fn detach(&mut self) {
        Value::detach(&mut self.nonce);
        Value::detach(&mut self.balances);
    }
}

#[allow(dead_code)]
pub struct AccountAt<'a> {
    pub nonce: <u64 as ViewField<'a, Account>>::Out,
    pub balances: <Map<String, u128> as ViewField<'a, Account>>::Out,
    _m: PhantomData<&'a ()>,
}

#[allow(dead_code)]
pub struct AccountAtMut<'a> {
    pub nonce: <u64 as ViewFieldMut<'a, Account>>::Out,
    pub balances: <Map<String, u128> as ViewFieldMut<'a, Account>>::Out,
    _m: PhantomData<&'a mut ()>,
}

impl Record for Account {
    type View<'a> = AccountAt<'a>;
    type ViewMut<'a> = AccountAtMut<'a>;
    fn __view<'a>(path: Path) -> AccountAt<'a> {
        AccountAt {
            nonce: <u64 as ViewField<'a, Account>>::__view_field(path.field(1)),
            balances: <Map<String, u128> as ViewField<'a, Account>>::__view_field(path.field(2)),
            _m: PhantomData,
        }
    }
    fn __view_mut_in<'a>(path: Path, anchor: usize) -> AccountAtMut<'a> {
        AccountAtMut {
            nonce: <u64 as ViewFieldMut<'a, Account>>::__view_field_mut(path.field(1), anchor),
            balances: <Map<String, u128> as ViewFieldMut<'a, Account>>::__view_field_mut(
                path.field(2), anchor,
            ),
            _m: PhantomData,
        }
    }
}

impl Removable for Account
where
    for<'x> u64: Removable,
    for<'x> Map<String, u128>: Removable,
{
    fn clear_children(path: &Path) {
        <u64 as Removable>::clear_children(&path.field(1));
        <Map<String, u128> as Removable>::clear_children(&path.field(2));
    }
    fn clear_children_some(path: &Path, budget: &mut u32) -> bool {
        if !<u64 as Removable>::clear_children_some(&path.field(1), budget) {
            return false;
        }
        if !<Map<String, u128> as Removable>::clear_children_some(&path.field(2), budget) {
            return false;
        }
        true
    }
}

// ---------------------------------------------------------------------------------------
// Hand-expanded #[near(contract_state(layout = v2))] struct Ex { #[key(2)] fee_bps: u16,
//                          #[key(3)] accounts: LookupMap<AccountId, Account> }
// ---------------------------------------------------------------------------------------

pub struct Ex {
    pub fee_bps: u16,
    pub accounts: LookupMap<AccountId, Account>,
}
impl Root for Ex {
    const MAX_SUBPATH: usize = p::max(
        p::field_subpath::<u16>(2),
        p::field_subpath::<LookupMap<AccountId, Account>>(3),
    );
    fn __encode_root(&self, orig: Option<&[u8]>) -> StdVec<u8> {
        let of = p::parse_orig(orig);
        let mut w = p::RecordWriter::root();
        p::encode_field(&mut w, of.as_ref(), 2, &self.fee_bps);
        p::encode_field(&mut w, of.as_ref(), 3, &self.accounts);
        w.finish(orig, |t| match t {
            2 => !<u16 as Value>::INLINE,
            3 => !<LookupMap<AccountId, Account> as Value>::INLINE,
            _ => true,
        })
    }
    fn __decode_root(bytes: &[u8], path: &Path) -> Result<Self, RecordError> {
        let f = p::parse(bytes)?;
        Ok(Self { fee_bps: p::field(&f, 2, Some(path))?, accounts: p::field(&f, 3, Some(path))? })
    }
    fn __bind_root(&mut self, path: &Path) {
        Value::bind(&mut self.fee_bps, &path.field(2));
        Value::bind(&mut self.accounts, &path.field(3));
    }
}

// ---------------------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------------------

fn alice() -> AccountId {
    "alice.near".parse().unwrap()
}
fn bob() -> AccountId {
    "bob.near".parse().unwrap()
}
fn big() -> StdVec<u8> {
    vec![7u8; 100 * 1024]
}
fn at<C: Default + Value>(tag: u32) -> C {
    let mut c = C::default();
    c.bind(&Path::root(DEFAULT_ROOT).field(tag));
    c
}
const KIB100: usize = 100 * 1024;

// ---------------------------------------------------------------------------------------
// LookupMap: cost table rows
// ---------------------------------------------------------------------------------------

#[test]
fn map_costs_match_table_beside_100kib_neighbor() {
    testing::use_mock();
    let mut m: LookupMap<u32, StdVec<u8>> = at(1);
    m.insert(1, big());
    m.insert(2, vec![1]);
    end_call();

    assert_eq!(m.get(&2).unwrap().len(), 1);
    let r = end_call();
    assert_eq!(r.rwd(), (1, 0, 0), "get: 1 read");
    assert_eq!(r.max_value_bytes(), 5, "borsh vec of 1 byte");

    m.insert(3, vec![1]);
    let r = end_call();
    assert_eq!(r.rwd(), (0, 1, 0), "insert new: 1 write, no existence read");

    m.insert(3, vec![2, 2]);
    let r = end_call();
    assert_eq!(r.rwd(), (0, 1, 0), "update existing via insert: 1 write");
    assert_eq!(r.ops[0].replaced_len, Some(5), "plus the old value's bytes (borsh vec of 1)");

    assert_eq!(m.replace(3, vec![3]), Some(vec![2, 2]));
    let r = end_call();
    assert_eq!(r.rwd(), (1, 1, 0), "replace: 1 read, 1 write");

    assert_eq!(m.remove(&3), Some(vec![3]));
    let r = end_call();
    assert_eq!(r.rwd(), (1, 0, 1), "remove: 1 read, 1 remove");
    assert!(r.max_value_bytes() < 16, "never touches the 100 KiB neighbor");

    m.discard(&2);
    let r = end_call();
    assert_eq!(r.rwd(), (0, 0, 1), "discard: 1 remove, no read");
    assert!(store().values().any(|v| v.len() == KIB100 + 4));
}

// ---------------------------------------------------------------------------------------
// Map: cost table rows, and removal locality
// ---------------------------------------------------------------------------------------

fn iter_fixture(neighbor: StdVec<u8>) -> Map<u32, StdVec<u8>> {
    testing::use_mock();
    let mut m: Map<u32, StdVec<u8>> = at(1);
    let _ = m.insert(1, vec![1]); // the entry under test, slot 0
    let _ = m.insert(2, neighbor); // the neighbor, last slot: it is the key that moves on removal
    end_call();
    m
}

#[test]
fn itermap_insert_and_update_costs() {
    let mut m = iter_fixture(big());
    let _ = m.insert(3, vec![3]);
    let r = end_call();
    assert_eq!(r.rwd(), (2, 4, 0), "insert new: index read, header read, 4 writes");

    let _ = m.insert(3, vec![4]);
    let r = end_call();
    assert_eq!(
        r.rwd(),
        (1, 1, 0),
        "update existing via insert: the table says 1 write, but insert cannot know the key \
         exists without the index read"
    );

    *m.get_mut(&3).unwrap() = vec![5];
    let r = end_call();
    assert_eq!(r.rwd(), (1, 1, 0), "update via get_mut: 1 read, 1 write");

    *m.entry(4).or_default() = vec![4];
    let r = end_call();
    assert_eq!(r.rwd(), (2, 4, 0), "entry().or_default() on a new key: entry read, header read, 4 writes");
}

#[test]
fn itermap_remove_cost_is_independent_of_neighbor_value() {
    let mut results = StdVec::new();
    for neighbor in [vec![9u8], big()] {
        let mut m = iter_fixture(neighbor);
        assert_eq!(m.remove(&1), Some(vec![1]));
        let r = end_call();
        assert_eq!(
            r.rwd(),
            (4, 3, 3),
            "entry read, index read, header read, last-slot read; moved slot write, moved index \
             write, header write; entry, index, last-slot removes"
        );
        assert!(r.max_value_bytes() <= 12, "no host call moves the neighbor's value (12: the header)");
        results.push((r.rwd(), r.read_bytes(), r.written_and_replaced_bytes()));
        // the moved key is still reachable and the vector is dense
        assert_eq!(m.len(), 1);
        assert_eq!(m.key_at(0), Some(2));
    }
    assert_eq!(results[0], results[1], "identical receipts beside a 1-byte and a 100 KiB neighbor");
}

#[test]
fn itermap_remove_last_key_has_no_moved_key() {
    let mut m = iter_fixture(vec![9]);
    m.remove(&2);
    let r = end_call();
    assert_eq!(r.rwd(), (3, 1, 3), "removing the last key skips the moved-key read and 2 writes");
    m.remove(&1);
    let r = end_call();
    assert_eq!(r.rwd(), (3, 1, 3), "emptying the map writes the header (len 0, revision kept, T4)");
}

#[test]
fn itermap_discard_skips_entry_read() {
    let mut m = iter_fixture(big());
    assert!(m.discard(&1));
    let r = end_call();
    assert_eq!(r.rwd(), (3, 3, 3));
}

#[test]
fn itermap_pagination_is_linear() {
    testing::use_mock();
    let mut m: Map<u32, u64> = at(1);
    for i in 0..20 {
        let _ = m.insert(i, i as u64);
    }
    end_call();
    let pg = m.page(None, 5).unwrap();
    assert_eq!(pg.items.len(), 5);
    assert_eq!(pg.next, Some(4));
    let r = end_call();
    assert_eq!(r.reads(), 1 + 2 * 5, "no cursor: header + 2 per item");

    let pg2 = m.page(pg.next.as_ref(), 5).unwrap();
    assert_eq!(pg2.items[0].0, 5);
    let r = end_call();
    assert_eq!(r.reads(), 2 + 2 * 5, "cursor: index + header + 2 per item (2n + 2)");

    let last = m.page(Some(&14), 50).unwrap();
    assert_eq!(last.items.len(), 5);
    assert_eq!(last.next, None);
    end_call();

    let _ = m.keys().take(7).count();
    assert_eq!(end_call().reads(), 1 + 7, "keys: 1 per key");
    let _ = m.iter().take(7).count();
    assert_eq!(end_call().reads(), 1 + 2 * 7, "iter: 2 per entry");
}

/// D8: a removed cursor key is an error value, not a panic, on every cursor-paged method.
#[test]
fn page_with_removed_cursor_returns_cursor_gone() {
    testing::use_mock();
    let mut m: Map<u32, u64> = at(1);
    for i in 0..3 {
        let _ = m.insert(i, 0);
    }
    end_call();
    m.discard(&1);
    end_call();
    assert_eq!(m.page(Some(&1), 10), Err(CursorGone));
    assert_eq!(m.page_with(Some(&1), 10, |_, v| *v), Err(CursorGone));
    assert_eq!(m.keys_page(Some(&1), 10), Err(CursorGone));
    assert_eq!(end_call().rwd(), (1, 0, 0), "one read, the cursor's index");
    assert!(CursorGone.as_ref().contains("cursor key no longer exists"));
    let mut s: Set<u32> = at(2);
    s.insert(1);
    s.remove(&1);
    assert_eq!(s.page(Some(&1), 10), Err(CursorGone));
}

/// D8: offset paging can SKIP a key after a removal behind the offset (the last key moves
/// into the hole, ahead of where the next page starts). Documented on `Map::page`.
#[test]
fn offset_paging_skips_a_key_after_a_removal_behind_the_offset() {
    testing::use_mock();
    let mut m: Map<u32, u64> = at(1);
    for i in 0..6 {
        let _ = m.insert(i, 0);
    }
    end_call();
    let p1: StdVec<u32> = m.keys().skip(0).take(3).collect(); // 0,1,2
    end_call();
    m.discard(&0); // key 5 moves to slot 0
    end_call();
    let p2: StdVec<u32> = m.keys().skip(3).take(3).collect(); // slots 3.. = 3,4
    let seen: StdVec<u32> = p1.into_iter().chain(p2).collect();
    assert_eq!(seen, vec![0, 1, 2, 3, 4], "key 5 is never returned");
}

/// Cursor pages skip a key when a removal between pages moves the last key into a gap
/// before the cursor. The spec says this; the test pins the exact failure shape.
#[test]
fn page_skips_moved_key_after_interleaved_removal() {
    testing::use_mock();
    let mut m: Map<u32, u64> = at(1);
    for i in 0..6 {
        let _ = m.insert(i, 0);
    }
    end_call();
    let p1 = m.page(None, 3).unwrap(); // 0,1,2
    end_call();
    m.discard(&0); // key 5 moves to slot 0
    end_call();
    let p2 = m.page(p1.next.as_ref(), 3).unwrap();
    let seen: StdVec<u32> =
        p1.items.iter().chain(p2.items.iter()).map(|(k, _)| *k).collect();
    assert_eq!(seen, vec![0, 1, 2, 3, 4], "key 5 is never returned");
}

#[test]
fn no_debt_counts_do_not_depend_on_history() {
    // fresh
    testing::use_mock();
    let mut m: Map<u32, u32> = at(1);
    let _ = m.insert(1_000_000, 1);
    let _ = m.insert(1_000_001, 1);
    end_call();
    let _ = m.insert(7, 7);
    let ins_fresh = end_call().rwd();
    m.remove(&1_000_000);
    let rm_fresh = end_call().rwd();

    // after 2,000 random-ish operations
    testing::use_mock();
    let mut m: Map<u32, u32> = at(1);
    let mut x: u32 = 12345;
    for _ in 0..2000 {
        x = x.wrapping_mul(1_103_515_245).wrapping_add(12345);
        let k = x % 300;
        if x & 1 == 0 {
            let _ = m.insert(k, x);
        } else {
            m.discard(&k);
        }
    }
    let _ = m.insert(1_000_000, 1);
    let _ = m.insert(1_000_001, 1);
    end_call();
    let _ = m.insert(7_000_000, 7);
    let ins_old = end_call().rwd();
    m.remove(&1_000_000);
    let rm_old = end_call().rwd();
    assert_eq!(ins_fresh, ins_old);
    assert_eq!(rm_fresh, rm_old);
}

// ---------------------------------------------------------------------------------------
// Vec, Lazy, LookupSet
// ---------------------------------------------------------------------------------------

#[test]
fn vec_costs() {
    testing::use_mock();
    let mut v: super::Vector<StdVec<u8>> = at(1);
    v.push(vec![1]);
    v.push(big());
    end_call();
    v.push(vec![3]);
    assert_eq!(end_call().rwd(), (1, 2, 0), "push: header read, 1 write, header write");
    assert_eq!(v.get(0).unwrap(), &vec![1]);
    assert_eq!(end_call().rwd(), (1, 0, 0));
    assert_eq!(v.pop(), Some(vec![3]));
    assert_eq!(
        end_call().rwd(),
        (2, 1, 1),
        "pop: the table says 1 remove + 1 header write; it also needs the header read and \
         (to return the value) the slot read"
    );
    let removed = v.swap_remove(0);
    assert_eq!(removed, vec![1]);
    let r = end_call();
    assert_eq!(r.rwd(), (3, 2, 1), "swap_remove: header, slot i, last slot reads; write i, header; remove last");
    assert_eq!(
        r.max_value_bytes(),
        KIB100 + 4,
        "swap_remove scales with the LAST element, as the table says: here it is 100 KiB"
    );
}

#[test]
fn lazy_costs() {
    testing::use_mock();
    let mut l: Lazy<StdVec<u8>> = at(5);
    l.set(big());
    assert_eq!(end_call().rwd(), (0, 1, 0));
    assert_eq!(l.get().unwrap().len(), KIB100);
    assert_eq!(end_call().rwd(), (1, 0, 0));
    l.remove();
    assert_eq!(end_call().rwd(), (0, 0, 1));
    // the Lazy's key is the field path itself, no kind byte
    assert!(store().is_empty());
}

#[test]
fn sets() {
    testing::use_mock();
    let mut s: LookupSet<u32> = at(1);
    let mut is: Set<u32> = at(2);
    s.insert(1);
    is.insert(1);
    is.insert(2);
    end_call();
    assert!(s.contains(&1));
    assert!(!s.contains(&2));
    assert_eq!(is.iter().collect::<StdVec<_>>(), vec![1, 2]);
    s.remove(&1);
    assert!(is.remove(&1));
    let r = end_call();
    assert_eq!(r.removes(), 1 + 3);
}

// ---------------------------------------------------------------------------------------
// The cache: one write per key, ordering, absent-key elision, views
// ---------------------------------------------------------------------------------------

#[test]
fn balance_updated_ten_times_is_one_write() {
    testing::use_mock();
    let mut m: Map<String, u128> = at(1);
    let _ = m.insert("usdt".into(), 0);
    end_call();
    for _ in 0..10 {
        *m.get_mut(&"usdt".to_string()).unwrap() += 1;
    }
    assert_eq!(end_call().rwd(), (1, 1, 0));
}

#[test]
fn insert_then_remove_of_key_seen_absent_costs_nothing_but_the_read() {
    testing::use_mock();
    let mut m: LookupMap<u32, u32> = at(1);
    assert!(m.get(&1).is_none());
    m.insert(1, 1);
    m.discard(&1);
    assert_eq!(end_call().rwd(), (1, 0, 0));
}

#[test]
fn insert_then_remove_of_unseen_key_costs_a_remove() {
    testing::use_mock();
    let mut m: LookupMap<u32, u32> = at(1);
    m.insert(1, 1);
    m.discard(&1);
    assert_eq!(
        end_call().rwd(),
        (0, 0, 1),
        "the cache cannot know the key was absent, so it must remove"
    );
}

#[test]
fn rewriting_the_same_value_is_elided_only_when_the_old_bytes_are_known() {
    testing::use_mock();
    let mut m: LookupMap<u32, u32> = at(1);
    m.insert(1, 5);
    end_call();
    m.insert(1, 5);
    assert_eq!(end_call().rwd(), (0, 1, 0), "blind insert writes");
    let _ = m.get(&1);
    m.insert(1, 5);
    assert_eq!(end_call().rwd(), (1, 0, 0), "after a read, an identical write is dropped");
}

#[test]
fn flush_is_key_ordered_with_root_last() {
    testing::use_mock();
    let mut ex: Ex = load_root();
    ex.fee_bps = 30;
    for who in ["zed.near", "alice.near", "mike.near", "bob.near"] {
        let id: AccountId = who.parse().unwrap();
        let a = ex.accounts.entry(id).or_default();
        a.nonce = 1;
        *a.balances.entry("usdt".into()).or_default() += 1;
    }
    let before = receipt().ops.len();
    assert!(flush_root(&ex));
    let r = receipt();
    let written: StdVec<StdVec<u8>> = r.ops[before..].iter().map(|o| o.key.clone()).collect();
    let root = vec![DEFAULT_ROOT];
    assert_eq!(written.last(), Some(&root), "root last");
    let rest = &written[..written.len() - 1];
    let mut sorted = rest.to_vec();
    sorted.sort();
    assert_eq!(rest, &sorted[..], "every other key in ascending byte order");
    let mut dedup = sorted.clone();
    dedup.dedup();
    assert_eq!(dedup.len(), sorted.len(), "each key once");
    assert!(rest.iter().all(|k| k[0] == DEFAULT_ROOT && k.len() > 1));
}

#[test]
fn view_call_writes_nothing() {
    testing::use_mock();
    let mut ex: Ex = load_root();
    *ex.accounts.entry(alice()).or_default().balances.entry("usdt".into()).or_default() = 5;
    flush_root(&ex);
    testing::begin_call();
    let ex: Ex = load_root();
    assert_eq!(ex.accounts.at(&alice()).balances.get(&"usdt".into()), Some(&5));
    testing::assert_view_clean();
    assert_eq!(receipt().writes(), 0);
}

#[test]
fn unchanged_root_is_not_rewritten() {
    testing::use_mock();
    let mut ex: Ex = load_root();
    ex.fee_bps = 1;
    flush_root(&ex);
    testing::begin_call();
    let mut ex: Ex = load_root();
    ex.accounts.try_insert(alice(), Account::default()).unwrap();
    assert!(!flush_root(&ex), "a collection write does not rewrite the root record");
    assert_eq!(receipt().rwd(), (1, 1, 0));
}

// ---------------------------------------------------------------------------------------
// The spec's "one call through the cache" scenario
// ---------------------------------------------------------------------------------------

/// Existing account alice with balances {usdt, dai, eth}. One call: deposit usdt twice,
/// bump the nonce, remove dai (not last, so eth moves into its slot).
#[test]
fn one_call_through_the_cache() {
    testing::use_mock();
    let mut ex: Ex = load_root();
    let a = ex.accounts.entry(alice()).or_default();
    for t in ["usdt", "dai", "eth"] {
        let _ = a.balances.insert(t.into(), 100);
    }
    flush_root(&ex);
    testing::begin_call();

    let mut ex: Ex = load_root(); // read 1: root
    let a = ex.accounts.entry(alice()).or_default(); // read 2: account record
    *a.balances.entry("usdt".into()).or_default() += 1; // read 3: usdt entry
    *a.balances.entry("usdt".into()).or_default() += 1; // cached
    a.nonce += 1;
    a.balances.remove(&"dai".to_string()); // reads 4-7: dai entry, dai index, header, last slot
    flush_root(&ex);
    let r = receipt();
    assert_eq!(r.reads(), 7);
    // writes: usdt entry, account record (nonce changed), slot 1 <- eth, eth index, header
    assert_eq!(r.writes(), 5);
    // removes: dai entry, dai index, last slot
    assert_eq!(r.removes(), 3);
    assert_eq!(r.sha256, 1 + 2 + 2 + 1, "alice; usdt twice (paths are recomputed per entry() call); dai entry + index; eth index");
}

// ---------------------------------------------------------------------------------------
// Records: tags, unknown fields, canonical form
// ---------------------------------------------------------------------------------------

fn alice_record_key() -> StdVec<u8> {
    Path::root(DEFAULT_ROOT).field(3).kind(kind::ENTRIES).entry(&alice()).as_bytes().to_vec()
}

#[test]
fn unknown_tags_survive_a_rewrite() {
    testing::use_mock();
    // A newer schema wrote nonce = 1 and tag 9 = [0xAA, 0xBB].
    let mut w = record::RecordWriter::new();
    w.push(1, borsh::to_vec(&1u64).unwrap());
    w.push(9, vec![0xAA, 0xBB]);
    seed(&alice_record_key(), &w.finish(None, |_| true));

    let mut ex: Ex = load_root();
    ex.accounts.get_mut(&alice()).unwrap().nonce = 2;
    flush_root(&ex);
    let stored = store()[&alice_record_key()].clone();
    let f = record::parse(&stored).unwrap();
    // API round 5: the rewrite also adds the `balances` kind marker the old bytes lacked.
    assert_eq!(
        f,
        vec![
            (1, record::Wire::Scalar, &borsh::to_vec(&2u64).unwrap()[..]),
            (2, record::Wire::Marker, &[record::marker::MAP][..]),
            (9, record::Wire::Scalar, &[0xAA, 0xBB][..])
        ]
    );
}

/// Hand-expanded scalar-only record `Slim { #[key(1)] nonce: u64 }` (owns no collections, so
/// `insert` stays a blind overwrite).
#[derive(Default, Debug)]
pub struct Slim {
    pub nonce: u64,
}
impl Leaf for Slim where for<'x> u64: Leaf {}
impl Value for Slim {
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

/// Item 8: a blind `insert` replaces the record whole and drops unknown tags, whether or not
/// the call read the key first (round 1: it depended on the read). `get_mut` preserves them.
#[test]
fn blind_insert_drops_unknown_tags_get_mut_keeps_them() {
    let newer = || {
        let mut w = record::RecordWriter::new();
        w.push(1, borsh::to_vec(&1u64).unwrap());
        w.push(9, vec![0xAA]);
        w.finish(None, |_| true)
    };
    let key = Path::root(DEFAULT_ROOT).field(7).kind(kind::ENTRIES).entry(&1u32);
    let tags = || {
        record::parse(&store()[key.as_bytes()]).unwrap().into_iter().map(|(t, _, _)| t).collect::<StdVec<_>>()
    };

    for read_first in [false, true] {
        testing::use_mock();
        seed(key.as_bytes(), &newer());
        let mut m: LookupMap<u32, Slim> = at(7);
        if read_first {
            let _ = m.get(&1);
        }
        m.insert(1, Slim { nonce: 2 });
        testing::flush();
        assert_eq!(tags(), vec![1], "blind insert drops tag 9 (read_first = {read_first})");
    }

    testing::use_mock();
    seed(key.as_bytes(), &newer());
    let mut m: LookupMap<u32, Slim> = at(7);
    m.get_mut(&1).unwrap().nonce = 2;
    testing::flush();
    assert_eq!(tags(), vec![1, 9], "read-modify-write keeps tag 9");
}

/// Item 15: insert of a record that owns collections is insert-new: 1 has_key + 1 write.
#[test]
fn insert_of_new_record_costs_a_has_key_and_a_write() {
    testing::use_mock();
    let mut ex: Ex = load_root();
    flush_root(&ex);
    testing::begin_call();
    let mut ex2: Ex = load_root();
    let before = receipt();
    ex2.accounts.try_insert(alice(), Account::default()).unwrap();
    flush_root(&ex2);
    let r = receipt();
    // D9 flag key: a miss at the record key also checks the flag key (a record being cleared
    // lives there), so a new record costs 2 existence checks.
    assert_eq!(r.has_keys() - before.has_keys(), 2);
    assert_eq!(r.writes() - before.writes(), 1);
    let _ = &mut ex;
}

/// Item 17: `contains_key` is one `storage_has_key` (no value bytes) when uncached, and a
/// later `get` still reads.
#[test]
fn contains_key_uses_has_key() {
    testing::use_mock();
    let mut m: LookupMap<u32, StdVec<u8>> = at(1);
    m.insert(1, big());
    end_call();
    assert!(m.contains_key(&1));
    assert!(!m.contains_key(&2));
    let r = end_call();
    assert_eq!((r.has_keys(), r.reads()), (2, 0));
    assert_eq!(r.max_value_bytes(), 0, "no value bytes moved for a 100 KiB value");

    assert!(m.contains_key(&1));
    assert_eq!(m.get(&1).unwrap().len(), KIB100);
    assert!(m.contains_key(&1), "cached now");
    let r = end_call();
    assert_eq!((r.has_keys(), r.reads()), (1, 1));

    let _ = m.get(&1);
    assert!(m.contains_key(&1));
    let r = end_call();
    assert_eq!((r.has_keys(), r.reads()), (0, 1), "after a read, contains_key is free");

    // has_key then remove/insert still flush correctly
    assert!(m.contains_key(&1));
    m.discard(&1);
    let r = end_call();
    assert_eq!(r.rwd(), (0, 0, 1));
    assert!(store().is_empty());
}

#[test]
fn missing_tag_reads_as_default() {
    testing::use_mock();
    seed(&alice_record_key(), &[]);
    let ex: Ex = load_root();
    let a = ex.accounts.get(&alice()).expect("an empty record is a present record");
    assert_eq!(a.nonce, 0);
    assert_eq!(a.balances.len(), 0);
}

#[test]
#[should_panic(expected = "TagZero")]
fn tag_zero_in_stored_record_panics() {
    testing::use_mock();
    // tag 0 is never a field (the D9 clearing flag is a separate key, not record bytes)
    seed(&alice_record_key(), &[0x00, 0x01, 0x05]);
    let ex: Ex = load_root();
    ex.accounts.get(&alice());
}

#[test]
#[should_panic(expected = "NonCanonicalVarint")]
fn non_canonical_varint_in_stored_record_panics() {
    testing::use_mock();
    seed(&alice_record_key(), &[0x81, 0x00, 0x00]);
    let ex: Ex = load_root();
    ex.accounts.get(&alice());
}

/// "A scalar whose Borsh length changes fails loudly at decode."
#[test]
#[should_panic(expected = "Field { tag: 1")]
fn scalar_length_change_fails_loudly() {
    testing::use_mock();
    let mut w = record::RecordWriter::new();
    w.push(1, borsh::to_vec(&1u32).unwrap()); // was u32, now u64
    seed(&alice_record_key(), &w.finish(None, |_| true));
    let ex: Ex = load_root();
    ex.accounts.get(&alice());
}

/// A field that changed from a scalar to a collection under the same tag: the stored bytes
/// are preserved as unknown, the collection starts empty. Silent, as the spec warns.
#[test]
fn scalar_to_collection_swap_is_silent() {
    testing::use_mock();
    let mut w = record::RecordWriter::new();
    w.push(2, borsh::to_vec(&77u128).unwrap()); // tag 2 used to be a scalar balance
    seed(&alice_record_key(), &w.finish(None, |_| true));
    let mut ex: Ex = load_root();
    let a = ex.accounts.get_mut(&alice()).unwrap();
    assert_eq!(a.balances.len(), 0);
    a.nonce = 1;
    flush_root(&ex);
    let f = record::parse(&store()[&alice_record_key()]).unwrap().into_iter().map(|(t, _, _)| t).collect::<StdVec<_>>();
    assert_eq!(f, vec![1, 2], "the old scalar bytes are carried forward as unknown");
}

#[test]
fn max_subpath_is_summed_along_the_schema() {
    // Ex: tag 3 (1) + kind (1) + h16 (16) + AccountId (68) + Account subpath
    // Account: tag 2 (1) + kind (1) + h16 (16) + String (516) + 0
    let h = crate::state::key::ENTRY_HASH_LEN;
    assert_eq!(<Account as Value>::MAX_SUBPATH, 1 + 1 + h + 516);
    assert_eq!(<Ex as Root>::MAX_SUBPATH, 1 + 1 + h + 68 + 2 + h + 516);
}

// ---------------------------------------------------------------------------------------
// Records with nested collections: at, or_default, Removable, binding
// ---------------------------------------------------------------------------------------

fn deposit(ex: &mut Ex, who: AccountId, token: &str, amount: u128) {
    let acct = ex.accounts.entry(who).or_default();
    *acct.balances.entry(token.into()).or_default() += amount;
}

#[test]
fn deposit_costs_and_parent_write_rule() {
    testing::use_mock();
    let mut ex: Ex = load_root();
    deposit(&mut ex, alice(), "usdt", 5);
    flush_root(&ex);
    let r = receipt();
    // root read, account read, account flag-key read (D9: a miss consults it), usdt entry read,
    // header read; account record write, 4 writes
    assert_eq!(r.rwd(), (5, 6, 0), "the 6th write is the root itself: absent before the first call, so it is created");

    testing::begin_call();
    let mut ex: Ex = load_root();
    deposit(&mut ex, alice(), "usdt", 5);
    flush_root(&ex);
    let r = receipt();
    assert_eq!(
        r.rwd(),
        (3, 1, 0),
        "existing account and token: root, account, entry reads; 1 write. The account record \
         is not rewritten: under the literal reading of 'written if anything below it was \
         written' this would be 2 writes"
    );
}

#[test]
fn balance_of_via_at_is_one_read_and_never_loads_the_record() {
    testing::use_mock();
    let mut ex: Ex = load_root();
    deposit(&mut ex, alice(), "usdt", 5);
    flush_root(&ex);
    testing::begin_call();
    let ex: Ex = load_root();
    let before = receipt().reads();
    let bal = ex.accounts.at(&alice()).balances.get(&"usdt".into()).copied().unwrap_or(0);
    assert_eq!(bal, 5);
    let r = receipt();
    assert_eq!(r.reads() - before, 1);
    assert!(r.ops.iter().all(|o| o.key != alice_record_key()));

    // scalars: `get` reads the record once (item 1: `at` navigates, `get` reads)
    assert_eq!(ex.accounts.get(&alice()).map(|a| a.nonce), Some(0));
    assert_eq!(ex.accounts.get(&alice()).map(|a| a.nonce), Some(0));
    assert_eq!(receipt().reads() - before, 2);

    // item 3: a read through a view outlives the view temporary
    let bal = ex.accounts.at(&alice()).balances.get(&"usdt".into());
    assert_eq!(bal, Some(&5));
}

#[test]
fn view_on_absent_key() {
    testing::use_mock();
    let ex: Ex = load_root();
    let v = ex.accounts.at(&bob());
    assert_eq!(v.balances.len(), 0);
    assert_eq!(v.balances.get(&"usdt".into()), None);
    assert!(ex.accounts.get(&bob()).is_none());
    assert!(v.balances.page(None, 10).unwrap().items.is_empty());
}

#[test]
fn removing_a_record_clears_its_collections() {
    testing::use_mock();
    let mut ex: Ex = load_root();
    for t in ["a", "b", "c"] {
        deposit(&mut ex, alice(), t, 1);
    }
    flush_root(&ex);
    let keys_before = store().len();
    assert_eq!(keys_before, 1 + 1 + 3 * 3 + 1, "root, account, 3x(entry, index, slot), header");

    testing::begin_call();
    let mut ex: Ex = load_root();
    let a = ex.accounts.remove(&alice()).unwrap();
    assert_eq!(a.nonce, 0);
    flush_root(&ex);
    let r = receipt();
    // reads: root, account, header, 3 slots. removes: 3x3 + header + account
    assert_eq!(r.rwd(), (6, 0, 11));
    assert_eq!(store().len(), 1, "only the root is left, nothing orphaned");
}

/// Item 16: the removed record's collections are detached, with their own message.
#[test]
#[should_panic(expected = "collection detached by remove; read it before removing")]
fn removed_records_collections_are_detached() {
    testing::use_mock();
    let mut ex: Ex = load_root();
    deposit(&mut ex, alice(), "a", 1);
    flush_root(&ex);
    testing::begin_call();
    let mut ex: Ex = load_root();
    let a = ex.accounts.remove(&alice()).unwrap();
    // Without detaching, this would write under alice's (cleared) path and orphan the key.
    a.balances.len();
}

/// D6/T1: `try_insert` returns the record bound to its key, so its collections are usable at
/// once; on `Map` it costs what a new-key `insert` costs.
#[test]
fn try_insert_returns_the_bound_record() {
    testing::use_mock();
    let mut m: Map<u32, Account> = at(11);
    let a = m.try_insert(1, Account { nonce: 3, ..Default::default() }).unwrap();
    let _ = a.balances.insert("usdt".into(), 5);
    a.nonce += 1;
    let r = end_call();
    assert_eq!(r.rwd(), (4, 8, 0), "outer: index + header reads, 4 writes; balances: the same");
    assert_eq!(m.get(&1).unwrap().nonce, 4);
    assert_eq!(m.at(&1).balances.get(&"usdt".to_string()), Some(&5));
    let mut lm: LookupMap<u32, Account> = at(12);
    lm.try_insert(1, Account::default()).unwrap().nonce = 2;
    let r = end_call();
    assert_eq!(
        (r.has_keys(), r.writes()),
        (2, 1),
        "LookupMap: 2 existence checks (record key, then its D9 flag key on the miss), 1 write"
    );
}

/// D6: `insert` always overwrites a `Leaf` value, including a scalar-only record, and
/// `Map::insert` reports whether the key was new.
#[test]
fn insert_always_overwrites_a_leaf_record() {
    testing::use_mock();
    let mut m: Map<u32, Slim> = at(13);
    assert!(m.insert(1, Slim { nonce: 1 }));
    end_call();
    assert!(!m.insert(1, Slim { nonce: 2 }), "existing key: overwritten, returns false");
    end_call();
    assert_eq!(m.get(&1).unwrap().nonce, 2);
    assert_eq!(m.len(), 1);
    end_call();
    let mut lm: LookupMap<u32, Slim> = at(14);
    lm.insert(1, Slim { nonce: 1 });
    lm.insert(1, Slim { nonce: 5 });
    let r = end_call();
    assert_eq!(r.rwd(), (0, 1, 0), "LookupMap::insert: no existence read");
    assert_eq!(lm.get(&1).unwrap().nonce, 5);
}

/// Item 15 / D6 / T1: `try_insert` over an existing key is `Err(Occupied)` instead of silently
/// inheriting the old record's children (round 1 behaviour): the index read only, the record
/// not read, nothing written. Plain `insert` does not compile for this record.
#[test]
fn try_insert_over_existing_record_is_occupied() {
    testing::use_mock();
    let mut ex: Ex = load_root();
    deposit(&mut ex, alice(), "a", 1);
    flush_root(&ex);
    testing::begin_call();
    let mut ex: Ex = load_root();
    let r = ex.accounts.try_insert(alice(), Account { nonce: 9, balances: Map::default() });
    assert_eq!(r.err(), Some(crate::state::Occupied));
    let r = end_call();
    assert_eq!(r.rwd(), (1, 0, 0), "alice's index slot only; the record is not read");
    assert_eq!(ex.accounts.get(&alice()).unwrap().nonce, 0, "unchanged");
}

/// T1: `try_insert` exists for leaf values too, and is the first-write-wins guard that
/// `require!(m.insert(..))` was not: an occupied key keeps its value.
#[test]
fn try_insert_on_leaf_values_keeps_the_first_write() {
    testing::use_mock();
    let mut m: Map<u32, u64> = at(15);
    assert_eq!(*m.try_insert(1, 10).unwrap(), 10);
    let r = end_call();
    assert_eq!(r.rwd(), (2, 4, 0), "as a new-key insert: index + header reads, 4 writes");
    assert!(m.try_insert(1, 20).is_err());
    let r = end_call();
    assert_eq!(r.rwd(), (1, 0, 0), "the index read only");
    assert_eq!(m.get(&1), Some(&10));
    end_call();
    let mut lm: LookupMap<u32, u64> = at(16);
    lm.try_insert(1, 10).unwrap();
    let r = end_call();
    assert_eq!((r.rwd(), r.has_keys()), ((0, 1, 0), 1), "LookupMap leaf: 1 check, 1 write");
    assert_eq!(lm.try_insert(1, 20).err(), Some(crate::state::Occupied));
    let r = end_call();
    assert_eq!((r.rwd(), r.has_keys()), ((0, 0, 0), 1), "1 check, no read, no write");
    assert_eq!(lm.get(&1), Some(&10));
}

/// Item 16: moving a record between keys is remove + rebuild. Re-inserting the removed value
/// panics instead of silently arriving with empty collections (round 1 behaviour).
#[test]
#[should_panic(expected = "build a new record instead of re-inserting the removed one")]
fn moving_a_record_between_keys_panics() {
    testing::use_mock();
    let mut ex: Ex = load_root();
    deposit(&mut ex, alice(), "a", 1);
    flush_root(&ex);
    testing::begin_call();
    let mut ex: Ex = load_root();
    let a = ex.accounts.remove(&alice()).unwrap();
    ex.accounts.try_insert(bob(), a).unwrap();
}

/// Item 16, the supported path: a removed record's scalars stay readable and a new record is
/// built from them.
#[test]
fn remove_then_rebuild() {
    testing::use_mock();
    let mut ex: Ex = load_root();
    deposit(&mut ex, alice(), "a", 1);
    ex.accounts.get_mut(&alice()).unwrap().nonce = 4;
    flush_root(&ex);
    testing::begin_call();
    let mut ex: Ex = load_root();
    let bal = *ex.accounts.at(&alice()).balances.get(&"a".into()).unwrap(); // read first
    let old = ex.accounts.remove(&alice()).unwrap();
    let new = ex.accounts.entry(bob()).or_default();
    new.nonce = old.nonce;
    let _ = new.balances.insert("a".into(), bal);
    flush_root(&ex);
    testing::begin_call();
    let ex: Ex = load_root();
    assert_eq!(ex.accounts.get(&bob()).unwrap().nonce, 4);
    assert_eq!(ex.accounts.at(&bob()).balances.get(&"a".into()), Some(&1));
}

/// Item 4: the sequential transfer pattern compiles, and the second `entry` on the same
/// record is free (cached).
#[test]
fn sequential_transfer_pattern() {
    testing::use_mock();
    let mut ex: Ex = load_root();
    deposit(&mut ex, alice(), "t", 10);
    deposit(&mut ex, bob(), "t", 0);
    flush_root(&ex);
    testing::begin_call();
    let mut ex: Ex = load_root();
    let t: String = "t".into();
    *ex.accounts.entry(alice()).or_default().balances.entry(t.clone()).or_default() -= 3;
    *ex.accounts.entry(bob()).or_default().balances.entry(t.clone()).or_default() += 3;
    let reads_after_transfer = receipt().reads();
    // touching the same records again costs nothing
    *ex.accounts.entry(alice()).or_default().balances.entry(t.clone()).or_default() -= 1;
    *ex.accounts.entry(bob()).or_default().balances.entry(t).or_default() += 1;
    assert_eq!(receipt().reads(), reads_after_transfer);
    flush_root(&ex);
    let r = receipt();
    // root, 2 records, 2 balance entries; 2 balance writes (records unchanged)
    assert_eq!(r.rwd(), (5, 2, 0));
}

/// `mem::take` on a `&mut Record` moves a bound collection handle out and leaves an unbound
/// one in the cache. Inserting the taken record elsewhere panics; reading the original
/// key's collections panics too.
#[test]
#[should_panic(expected = "already bound")]
fn mem_take_breaks_binding() {
    testing::use_mock();
    let mut ex: Ex = load_root();
    deposit(&mut ex, alice(), "a", 1);
    let taken = core::mem::take(ex.accounts.get_mut(&alice()).unwrap());
    ex.accounts.try_insert(bob(), taken).unwrap();
}

#[test]
#[should_panic(expected = "not bound")]
fn mem_take_leaves_unbound_collection_behind() {
    testing::use_mock();
    let mut ex: Ex = load_root();
    deposit(&mut ex, alice(), "a", 1);
    let _taken = core::mem::take(ex.accounts.get_mut(&alice()).unwrap());
    ex.accounts.get(&alice()).unwrap().balances.len();
}

#[test]
#[should_panic(expected = "not bound")]
fn touching_an_unbound_collection_panics() {
    testing::use_mock();
    let a = Account::default();
    a.balances.len();
}

/// Clearing a big record with `clear_some` bounds work per call.
#[test]
fn bounded_clear() {
    testing::use_mock();
    let mut m: Map<u32, u32> = at(1);
    for i in 0..10 {
        let _ = m.insert(i, i);
    }
    end_call();
    assert!(m.clear_some(4));
    let r = end_call();
    assert_eq!(r.rwd(), (1 + 4, 1, 12), "header + 4 slot reads; header write; 3 removes each");
    assert!(m.clear_some(4));
    end_call();
    assert!(!m.clear_some(4));
    end_call();
    // T4: only the header is left, len 0 and the revision bumped once per `clear_some` call.
    assert_eq!(store().len(), 1);
    assert_eq!(m.len(), 0);
    assert_eq!(m.revision(), 3);
}

#[test]
fn grouped_map_groups_are_independent_itermaps() {
    testing::use_mock();
    let mut g: GroupedMap<u32, u64, u64> = at(4);
    let _ = g.group_mut(&1).insert(10, 1);
    let _ = g.group_mut(&1).insert(11, 1);
    let _ = g.group_mut(&2).insert(10, 2);
    end_call();
    assert_eq!(g.group(&1).len(), 2);
    assert_eq!(g.group(&2).get(&10), Some(&2));
    end_call();
    g.group_mut(&2).remove(&10);
    let r = end_call();
    assert_eq!(r.rwd(), (3, 1, 3), "last entry of a group: the header stays with len 0 (T4)");
    // group(&1) keys are under 04 00 h16(1) borsh(1) 00 ...
    let k = Path::root(DEFAULT_ROOT).field(4).kind(kind::ENTRIES).entry(&1u32).kind(kind::ENTRIES).entry(&10u64);
    assert!(store().contains_key(k.as_bytes()));
}

/// Cache memory: a loop over n entries holds n decoded values plus their bookkeeping slots.
#[test]
fn cache_holds_every_touched_key_for_the_call() {
    testing::use_mock();
    let mut m: Map<u32, StdVec<u8>> = at(1);
    for i in 0..100 {
        let _ = m.insert(i, vec![0; 1024]);
    }
    end_call();
    let n = m.iter().fold(0, |n, _| n + 1); // `count()` alone reads only the header
    assert_eq!(n, 100);
    assert_eq!(testing::cache_slots(), 1 + 2 * 100, "header + slot + entry per item, none freed");
}

// ---------------------------------------------------------------------------------------
// Revision 3
// ---------------------------------------------------------------------------------------

#[test]
fn map_insert_reports_whether_the_key_was_new() {
    testing::use_mock();
    let mut m: Map<u32, u32> = at(1);
    assert!(m.insert(1, 10));
    assert!(!m.insert(1, 11));
    assert_eq!(m.get(&1), Some(&11));
    end_call();
    assert!(!m.insert(1, 12));
    assert_eq!(end_call().rwd(), (1, 1, 0), "existing key: index read, entry write");
}

#[test]
fn set_insert_of_an_existing_key_writes_nothing_and_set_has_map_parity() {
    testing::use_mock();
    let mut s: Set<u32> = at(2);
    for i in 0..5 {
        assert!(s.insert(i));
    }
    end_call();
    assert!(!s.insert(3));
    assert_eq!(end_call().rwd(), (1, 0, 0), "index read only");
    assert_eq!(s.key_at(4), Some(4));
    let p = s.page(None, 2).unwrap();
    assert_eq!((p.keys, p.next), (vec![0, 1], Some(1)));
    end_call();
    let got: StdVec<u32> = s.iter().skip(3).take(2).collect();
    assert_eq!(got, vec![3, 4]);
    assert_eq!(end_call().rwd(), (3, 0, 0), "header + 2 key slots; skip reads nothing");
}

#[test]
fn skip_and_count_are_free_on_map_and_vector() {
    testing::use_mock();
    let mut m: Map<u32, u32> = at(3);
    let mut v: Vector<u32> = at(4);
    for i in 0..10 {
        let _ = m.insert(i, i);
        v.push(i);
    }
    end_call();
    let x: StdVec<(u32, u32)> = m.iter().skip(8).map(|(k, v)| (k, *v)).collect();
    assert_eq!(x, vec![(8, 8), (9, 9)]);
    assert_eq!(end_call().rwd(), (5, 0, 0), "header + 2 x (key slot, entry)");
    let y: StdVec<u32> = v.iter().skip(9).copied().collect();
    assert_eq!(y, vec![9]);
    assert_eq!(end_call().rwd(), (2, 0, 0), "header + 1 slot");
    assert_eq!(m.iter().count(), 10);
    assert_eq!(end_call().rwd(), (1, 0, 0), "count reads only the header");
}

#[test]
fn unit_valued_remove_costs_what_discard_costs() {
    testing::use_mock();
    let mut a: Map<u32, ()> = at(5);
    let mut b: Map<u32, ()> = at(6);
    for i in 0..3 {
        let _ = a.insert(i, ());
        let _ = b.insert(i, ());
    }
    end_call();
    assert_eq!(a.remove(&0), Some(()));
    let ra = end_call();
    assert!(b.discard(&0));
    let rb = end_call();
    assert_eq!(ra.rwd(), rb.rwd());
    assert_eq!(a.remove(&0), None);
}

#[test]
fn page_with_maps_record_values() {
    testing::use_mock();
    let mut m: Map<u32, Account> = at(7);
    for i in 0..3u32 {
        m.entry(i).or_default().nonce = u64::from(i) * 10;
    }
    end_call();
    let p = m.page_with(None, 2, |k, a| (*k, a.nonce)).unwrap();
    assert_eq!(p.items, vec![(0, (0, 0)), (1, (1, 10))]);
    assert_eq!(p.next, Some(1));
}

#[test]
fn at_mut_writes_below_the_record_without_decoding_it() {
    testing::use_mock();
    let mut m: Map<u32, Account> = at(8);
    m.entry(1).or_default().nonce = 7;
    end_call();
    let rec = Path::root(DEFAULT_ROOT).field(8).kind(kind::ENTRIES).entry(&1u32);
    // Round 7: the first write into the empty `balances` (no header) is the A3 first-write
    // case: 1 read of the record, whose marker is already there, so no rewrite.
    let _ = m.at_mut(&1).unwrap().balances.insert("usdt".into(), 5);
    let r = end_call();
    assert_eq!(r.has_keys(), 1, "the record's existence; D9's flag key is not checked on a hit");
    assert_eq!(r.rwd(), (3, 4, 0), "the record, balances index + header reads, 4 writes");
    assert_eq!(r.ops.iter().filter(|o| o.key == rec.as_bytes()).count(), 2, "has_key + 1 read of the record, no write");
    // Every later write below it: the record is neither read nor rewritten.
    let _ = m.at_mut(&1).unwrap().balances.insert("usdc".into(), 6);
    let r = end_call();
    assert_eq!(r.has_keys(), 1);
    assert_eq!(r.rwd(), (2, 4, 0), "balances index + header reads, 4 writes; the record is neither read nor rewritten");
    assert_eq!(m.get(&1).unwrap().balances.get(&"usdt".to_string()), Some(&5));
    assert_eq!(m.get(&1).unwrap().nonce, 7);
}

/// D9, API round 5: `remove_step` budgets host removes across nested maps. A record whose
/// `balances` holds 5 entries (3 removes each) takes 3 calls at n = 7: 2 entries plus the move
/// to the flag key (7), 2 entries (6), then the last entry, the balances header and the record
/// with its index and key slot (3 + 1 + 3 = 7). The record sits at its flag key (record key
/// absent) until the last call, and the outer map's key vector drops the key only then.
#[test]
fn remove_step_budgets_across_calls() {
    testing::use_mock();
    let mut m: Map<u32, Account> = at(15);
    let a = m.entry(1).or_default();
    for i in 0..5 {
        let _ = a.balances.insert(format!("t{i}"), 1);
    }
    m.entry(2).or_default().nonce = 1;
    end_call();
    let rec = Path::root(DEFAULT_ROOT).field(15).kind(kind::ENTRIES).entry(&1u32).as_bytes().to_vec();
    let flag = Path::root(DEFAULT_ROOT).field(15).kind(kind::ENTRIES).entry(&1u32).flag();
    let before = store()[&rec].clone();
    assert_eq!(m.remove_step(&1, nz(7)), Removal::Pending);
    assert_eq!(end_call().removes(), 7, "2 entries and the record key");
    assert!(!store().contains_key(&rec), "the record key is absent while clearing");
    assert_eq!(store()[flag.as_bytes()], before, "the record's bytes, unchanged, at the flag key");
    assert_eq!(m.len(), 2, "still indexed while clearing");
    assert!(m.contains_key(&1) && m.get(&1).is_some(), "a record being cleared still reads");
    end_call();
    assert_eq!(m.remove_step(&1, nz(7)), Removal::Pending);
    assert_eq!(end_call().removes(), 6, "2 entries; a third would need 9");
    assert_eq!(m.remove_step(&1, nz(7)), Removal::Done, "last entry, then the record");
    assert_eq!(end_call().removes(), 7, "entry, header, record (flag key), index, key slot");
    assert!(!store().contains_key(&rec));
    assert_eq!(store().keys().filter(|k| k.starts_with(&rec)).count(), 0);
    assert_eq!(m.len(), 1);
    assert_eq!(m.key_at(0), Some(2));
    // The minimum, n = 3, on a LookupMap record with one entry: the first call can only move
    // the record (2 left, an entry needs 3), the second removes the entry (3; the header would
    // be a 4th), the third the header and the record.
    let mut lm: LookupMap<u32, Account> = at(16);
    let _ = lm.entry(1).or_default().balances.insert("a".into(), 1);
    end_call();
    let mut per_call = StdVec::new();
    loop {
        let more = lm.remove_step(&1, nz(3)) == Removal::Pending;
        per_call.push(end_call().removes());
        if !more {
            break;
        }
    }
    assert_eq!(per_call, [1, 3, 2]);
    assert!(lm.get(&1).is_none());
}

/// API round 5: `n` is a `NonZeroU32` (0 does not compile), and 1 or 2 panic: one map entry is
/// 3 host removes that cannot be split, so a smaller budget could never progress.
#[test]
fn remove_step_below_the_minimum_panics() {
    testing::use_mock();
    let mut lm: LookupMap<u32, Account> = at(17);
    let _ = lm.entry(1).or_default().balances.insert("a".into(), 1);
    end_call();
    for n in [1, 2] {
        let e = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| lm.remove_step(&1, nz(n))))
            .expect_err("budget below the minimum");
        assert!(e.downcast_ref::<String>().is_some_and(|m| m.contains("must be at least 3")));
        testing::begin_call();
    }
    assert_eq!(MIN_REMOVE_STEP, 3);
}

/// D9 flag key: every path that ends a clear early (`replace`, `remove`, `discard`) removes
/// the flag key; `try_insert` over a record being cleared panics; iteration and `get` still
/// see the record; a flag key never outlives its clear.
#[test]
fn flag_key_is_dropped_by_replace_remove_and_discard() {
    fn flagged_lookup(tag: u32) -> (LookupMap<u32, Account>, StdVec<u8>, StdVec<u8>) {
        let mut lm: LookupMap<u32, Account> = at(tag);
        let a = lm.entry(1).or_default();
        a.nonce = 4;
        let _ = a.balances.insert("a".into(), 1);
        let _ = a.balances.insert("b".into(), 1);
        end_call();
        // 4: the move to the flag key and one entry
        assert_eq!(lm.remove_step(&1, nz(4)), Removal::Pending);
        end_call();
        let rec = Path::root(DEFAULT_ROOT).field(tag).kind(kind::ENTRIES).entry(&1u32);
        let (rec, flag) = (rec.as_bytes().to_vec(), rec.flag().as_bytes().to_vec());
        assert!(!store().contains_key(&rec) && store().contains_key(&flag));
        (lm, rec, flag)
    }
    testing::use_mock();

    // replace: clears without a bound, the new record goes to the record key
    let (mut lm, rec, flag) = flagged_lookup(20);
    let old = lm.replace(1, Account { nonce: 9, ..Default::default() }).expect("was present");
    assert_eq!(old.nonce, 4);
    end_call();
    assert!(store().contains_key(&rec) && !store().contains_key(&flag));
    assert_eq!(lm.get(&1).unwrap().nonce, 9);
    assert_eq!(lm.at(&1).balances.len(), 0);
    end_call();

    // remove: finds the record at its flag key and removes it there
    let (mut lm, rec, flag) = flagged_lookup(21);
    assert_eq!(lm.remove(&1).map(|a| a.nonce), Some(4));
    end_call();
    assert_eq!(store().keys().filter(|k| k.starts_with(&rec)).count(), 0);
    let _ = flag;

    // discard: since API round 5 it reads the record first (the unknown-collection check), so
    // it knows the flag key holds it and removes only that key (was blind: both keys, 6)
    let (mut lm, rec, _) = flagged_lookup(22);
    lm.discard(&1);
    let r = end_call();
    assert_eq!(store().keys().filter(|k| k.starts_with(&rec)).count(), 0);
    assert_eq!(
        r.rwd().2,
        5,
        "the remaining balance's entry, index, slot and the balances header; then the flag key"
    );

    // try_insert over a record being cleared panics, and so does `entry`
    let (mut lm, _, _) = flagged_lookup(23);
    let e = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _ = lm.try_insert(1, Account::default());
    }))
    .expect_err("try_insert over a clearing record");
    assert!(e.downcast_ref::<String>().is_some_and(|m| m.contains("is being cleared")));
    testing::begin_call();
    let e = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        lm.entry(1).or_default();
    }))
    .expect_err("entry over a clearing record");
    assert!(e.downcast_ref::<String>().is_some_and(|m| m.contains("is being cleared")));
    testing::begin_call();

    // Map: iteration still yields a record being cleared (its key stays indexed)
    let mut m: Map<u32, Account> = at(24);
    let a = m.entry(1).or_default();
    a.nonce = 5;
    let _ = a.balances.insert("a".into(), 1);
    let _ = a.balances.insert("b".into(), 1);
    end_call();
    assert_eq!(m.remove_step(&1, nz(4)), Removal::Pending);
    end_call();
    assert_eq!(m.iter().map(|(k, v)| (k, v.nonce)).collect::<StdVec<_>>(), vec![(1, 5)]);
    end_call();
}

/// D7: `at_mut` on an absent record is `None` (std convention), and creates nothing.
#[test]
fn at_mut_on_an_absent_record_is_none() {
    testing::use_mock();
    let mut m: Map<u32, Account> = at(9);
    assert!(m.at_mut(&9).is_none());
    let mut lm: LookupMap<u32, Account> = at(10);
    assert!(lm.at_mut(&9).is_none());
    let r = end_call();
    assert_eq!(r.reads(), 0, "at_mut reads nothing");
    assert_eq!(r.has_keys(), 4, "per map: the record key, then (a miss) its D9 flag key");
    assert_eq!(r.keys_written().len(), 0, "nothing created");
}

/// T4: every page carries the map's revision, from the header it reads anyway. Two pages with
/// no removal in between have equal revisions; a removal in between changes it, even one that
/// empties the map before the cursor key comes back (the header keeps the revision at len 0).
/// Removing the record that owns the map removes the header too.
#[test]
fn page_revision_changes_only_across_a_removal() {
    testing::use_mock();
    let mut m: Map<u32, u64> = at(31);
    for i in 0..4 {
        let _ = m.insert(i, i as u64);
    }
    end_call();
    let p1 = m.page(None, 2).unwrap();
    let r = end_call();
    assert_eq!(r.rwd(), (1 + 4, 0, 0), "header + 2 slots + 2 entries: the revision is free");
    assert_eq!(p1.revision, 0);
    let _ = m.insert(9, 9);
    end_call();
    let p2 = m.page(p1.next.as_ref(), 2).unwrap();
    assert_eq!(p2.revision, p1.revision, "an insert is not a removal");
    assert_eq!(m.keys_page(None, 1).unwrap().revision, p1.revision);
    end_call();

    m.remove(&3);
    let r = end_call();
    assert_eq!(r.rwd(), (4, 3, 3), "a removal costs what it did: the header write carries it");
    let p3 = m.page(p1.next.as_ref(), 2).unwrap();
    assert_ne!(p3.revision, p1.revision, "a removal between pages changes the revision");
    assert_eq!(p3.revision, 1);
    end_call();

    // Empty the map, then bring the cursor key back: still a different revision.
    let p4 = m.page(None, 1).unwrap();
    let cursor = p4.next.clone().unwrap();
    end_call();
    m.clear();
    end_call();
    assert_eq!(m.len(), 0);
    let _ = m.insert(cursor, 7);
    let _ = m.insert(100, 7);
    end_call();
    let p5 = m.page(Some(&cursor), 2).unwrap();
    assert_ne!(p5.revision, p4.revision, "emptied and refilled: the revision did not reset");
    end_call();
    let hdr = Path::root(DEFAULT_ROOT).field(31).kind(kind::HEADER);
    assert_eq!(store()[hdr.as_bytes()].len(), 12, "len ++ revision once a removal happened");

    // A map that never had a removal keeps the 4-byte header.
    let mut fresh: Map<u32, u64> = at(32);
    let _ = fresh.insert(1, 1);
    end_call();
    let hdr = Path::root(DEFAULT_ROOT).field(32).kind(kind::HEADER);
    assert_eq!(store()[hdr.as_bytes()], 1u32.to_le_bytes().to_vec());

    // The owner's removal removes the header, revision and all.
    let mut owners: Map<u32, Account> = at(33);
    let a = owners.try_insert(1, Account::default()).unwrap();
    let _ = a.balances.insert("x".into(), 1);
    end_call();
    let _ = owners.get_mut(&1).unwrap().balances.remove(&"x".to_string());
    end_call();
    let below = Path::root(DEFAULT_ROOT).field(33).kind(kind::ENTRIES).entry(&1u32).field(2);
    assert!(store().keys().any(|k| k.starts_with(below.as_bytes())), "the emptied balances header");
    owners.remove(&1);
    end_call();
    assert!(!store().keys().any(|k| k.starts_with(below.as_bytes())), "nothing left below the record");
}

/// Review F1: a record left mid-clear by an earlier receipt lives at its flag key. In a fresh
/// receipt, a blind `discard` then a reinsert (`try_insert`, `entry().or_default()`) must still
/// remove the flag key at the flush; otherwise both keys stay and removing the replacement
/// later resurrects the old record from the flag key.
#[test]
fn reinsert_after_blind_discard_of_a_clearing_record_drops_its_flag_key() {
    fn flagged(tag: u32) -> (LookupMap<u32, Account>, StdVec<u8>, StdVec<u8>) {
        let mut lm: LookupMap<u32, Account> = at(tag);
        let a = lm.entry(1).or_default();
        a.nonce = 4;
        let _ = a.balances.insert("a".into(), 1);
        let _ = a.balances.insert("b".into(), 1);
        end_call();
        // 4: the move to the flag key and one entry
        assert_eq!(lm.remove_step(&1, nz(4)), Removal::Pending);
        end_call();
        let rec = Path::root(DEFAULT_ROOT).field(tag).kind(kind::ENTRIES).entry(&1u32);
        let (rec, flag) = (rec.as_bytes().to_vec(), rec.flag().as_bytes().to_vec());
        assert!(!store().contains_key(&rec) && store().contains_key(&flag));
        (lm, rec, flag)
    }
    testing::use_mock();
    for (tag, via_entry) in [(40, false), (41, true)] {
        let (mut lm, rec, flag) = flagged(tag);
        // receipt 2: discard (since API round 5 it reads the record, the unknown-collection
        // check, so it finds it at the flag key), then a new record at the same key
        lm.discard(&1);
        if via_entry {
            lm.entry(1).or_default().nonce = 7;
        } else {
            lm.try_insert(1, Account { nonce: 7, ..Default::default() }).unwrap();
        }
        end_call();
        assert!(store().contains_key(&rec), "the replacement is at the record key");
        assert!(!store().contains_key(&flag), "the old record's flag key is gone");
        assert_eq!(lm.get(&1).unwrap().nonce, 7);
        end_call();
        // receipt 3: removing the replacement leaves nothing to resurrect
        assert_eq!(lm.remove(&1).map(|a| a.nonce), Some(7));
        end_call();
        assert!(lm.get(&1).is_none(), "no old record comes back from the flag key");
        assert_eq!(store().keys().filter(|k| k.starts_with(&rec)).count(), 0);
        end_call();
    }

    // The same through `Map` (whose `discard` also reads the record since API round 5). The
    // F1 rule stays reachable through `try_insert`, which writes a record blind.
    let mut m: Map<u32, Account> = at(42);
    let a = m.entry(1).or_default();
    let _ = a.balances.insert("a".into(), 1);
    let _ = a.balances.insert("b".into(), 1);
    end_call();
    assert_eq!(m.remove_step(&1, nz(4)), Removal::Pending);
    end_call();
    let rec = Path::root(DEFAULT_ROOT).field(42).kind(kind::ENTRIES).entry(&1u32);
    let (rec, flag) = (rec.as_bytes().to_vec(), rec.flag().as_bytes().to_vec());
    assert!(store().contains_key(&flag));
    assert!(m.discard(&1));
    m.try_insert(1, Account { nonce: 8, ..Default::default() }).unwrap();
    end_call();
    assert!(store().contains_key(&rec) && !store().contains_key(&flag));
    assert_eq!(m.remove(&1).map(|a| a.nonce), Some(8));
    end_call();
    assert!(m.get(&1).is_none());
    assert_eq!(store().keys().filter(|k| k.starts_with(&rec)).count(), 0);
}

/// Review F2, pinned as a hazard: a record written by a pre-flag-key prototype build carries
/// the old leading `00 00` clearing flag in its bytes and sits at its record key. `get` and
/// `get_mut` reject it (`TagZero`), but `at_mut` only checks that the record key exists, so it
/// permits writes below a record that build was halfway through clearing, into a collection
/// that already exists. Storage written by earlier prototype builds must be reset; nothing
/// detects it cheaply. Round 7: the FIRST write of a collection below it now reads the record
/// (the A3 first-write rule) and fails closed on the same `TagZero`.
#[test]
fn hazard_old_in_record_flag_passes_at_mut_but_fails_get() {
    testing::use_mock();
    let rec = Path::root(DEFAULT_ROOT).field(43).kind(kind::ENTRIES).entry(&1u32);
    let mut w = record::RecordWriter::new();
    w.push(1, borsh::to_vec(&4u64).unwrap());
    let mut old = vec![0u8, 0];
    old.extend(w.finish(None, |_| true));
    seed(rec.as_bytes(), &old);
    let mut lm: LookupMap<u32, Account> = at(43);
    let first = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _ = lm.at_mut(&1).expect("the record key exists").balances.insert("x".into(), 1);
    }));
    let msg = first.expect_err("the first write below it reads the record and fails closed");
    assert!(msg.downcast_ref::<String>().is_some_and(|m| m.contains("TagZero")));
    testing::begin_call();
    // the hazard that remains: `balances` already holds an entry (written before the old flag)
    let hdr = rec.field(2).kind(kind::HEADER);
    seed(hdr.as_bytes(), &1u32.to_le_bytes());
    let _ = lm.at_mut(&1).expect("the record key exists").balances.insert("x".into(), 1);
    end_call();
    assert_eq!(lm.at(&1).balances.get(&"x".to_string()), Some(&1), "written below it");
    end_call();
    let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| lm.get(&1).map(|a| a.nonce)));
    let msg = r.expect_err("get fails closed on the old prefix");
    assert!(msg.downcast_ref::<String>().is_some_and(|m| m.contains("TagZero")));
}

/// Review F3 edge: after `remove_step` flags a record, a `replace` and a new `remove_step` in
/// the same call relocate the NEW record's encoding, not the old stored bytes.
#[test]
fn replaced_then_flagged_record_relocates_the_new_value() {
    testing::use_mock();
    let mut lm: LookupMap<u32, Account> = at(44);
    let a = lm.entry(1).or_default();
    a.nonce = 4;
    let _ = a.balances.insert("a".into(), 1);
    let _ = a.balances.insert("b".into(), 1);
    end_call();
    assert_eq!(lm.remove_step(&1, nz(4)), Removal::Pending);
    let _ = lm.replace(1, Account { nonce: 9, ..Default::default() });
    let _ = lm.at_mut(&1).unwrap().balances.insert("c".into(), 1);
    // 3: the move (1) and the new entry "c" (2: its index was never stored), not its header
    assert_eq!(lm.remove_step(&1, nz(3)), Removal::Pending);
    end_call();
    let flag = Path::root(DEFAULT_ROOT).field(44).kind(kind::ENTRIES).entry(&1u32).flag();
    let stored = store()[flag.as_bytes()].clone();
    let fields = record::parse(&stored).unwrap();
    assert_eq!(fields.iter().find(|(t, _, _)| *t == 1).map(|(_, _, b)| b.to_vec()), Some(borsh::to_vec(&9u64).unwrap()));
}

/// T4: clearing an empty map that never had a header costs no header op: the header read
/// already says it is absent, so deleting it is free. `remove_step` and `remove` of a record
/// with an empty `balances`, and `clear()` on an empty map.
#[test]
fn clearing_an_empty_map_without_a_header_writes_nothing_for_it() {
    testing::use_mock();
    let mut lm: LookupMap<u32, Account> = at(45);
    lm.try_insert(1, Account::default()).unwrap();
    lm.try_insert(2, Account::default()).unwrap();
    end_call();
    assert_eq!(lm.remove_step(&1, nz(3)), Removal::Done);
    let r = end_call();
    assert_eq!(r.rwd(), (2, 0, 1), "record read + balances header read; record remove");
    assert!(lm.remove(&2).is_some());
    let r = end_call();
    assert_eq!(r.rwd(), (2, 0, 1), "record read + balances header read; record remove");
    let mut m: Map<u32, u64> = at(46);
    m.clear();
    let r = end_call();
    assert_eq!(r.rwd(), (1, 0, 0), "the header read only");
}

// ---------------------------------------------------------------------------------------
// Audit round (Oct 17, Codex claims). A1: `cache::get` retags `&mut V` on a shared lookup.
//
// The traces below compile in safe code: `Map::get(&self) -> Option<&V>`, so two shared
// lookups may overlap. API round 4 fixed `cache::get` (a shared type check and a pointer cast,
// no `&mut`), so they now run in plain `cargo test` too, where they only check values; the
// aliasing verdict needs Miri, under both models:
//
//   NEAR_SANDBOX_BIN_PATH=/usr/bin/true MIRIFLAGS=-Zmiri-ignore-leaks \
//     cargo +nightly miri test -p near-sdk --lib --features unstable,unit-testing -- state::tests::audit_
//
// (add `-Zmiri-tree-borrows` to MIRIFLAGS for the Tree Borrows verdict).
// ---------------------------------------------------------------------------------------

fn audit_a1_fixture(tag: u32, n: u32) -> Map<u32, u128> {
    testing::use_mock();
    let mut m: Map<u32, u128> = at(tag);
    for i in 0..n {
        m.insert(i, 1000 + u128::from(i));
    }
    end_call();
    m
}

/// A1, same key: `&V` from `get(k)` held across a second `get(k)`, then used.
#[test]
fn audit_a1_shared_ref_held_across_second_get_of_same_key() {
    let m = audit_a1_fixture(40, 1);
    let first: &u128 = m.get(&0).unwrap();
    let second: &u128 = m.get(&0).unwrap();
    assert_eq!(*second, 1000);
    assert_eq!(*first, 1000, "use of the first reference after the second get");
    end_call();
}

/// A1, other keys: `&V` held across 200 `get`s of other keys (new slots, BTreeMap node splits),
/// then used. Slots and values are separate boxes, so this exercises "could reallocate".
#[test]
fn audit_a1_shared_ref_held_across_gets_of_other_keys() {
    let m = audit_a1_fixture(41, 200);
    let first: &u128 = m.get(&0).unwrap();
    let mut sum = 0u128;
    for i in 1..200 {
        sum += *m.get(&i).unwrap();
    }
    assert!(sum > 0);
    assert_eq!(*first, 1000);
    end_call();
}

/// A1, realistic surface: references collected from `iter()`, then a lookup of one of those
/// keys, then a use of the collected reference.
#[test]
fn audit_a1_iter_refs_then_get_of_same_key() {
    let m = audit_a1_fixture(42, 3);
    let all: StdVec<(u32, &u128)> = m.iter().collect();
    let again = m.get(&all[0].0).unwrap();
    assert_eq!(*again, 1000);
    assert_eq!(*all[0].1, 1000);
    end_call();
}

// ---------------------------------------------------------------------------------------
// Audit round (Oct 17, Codex claims). A2: `mem::take` of a collection field escapes a bound
// handle; `at_mut` / `get_mut` then recreate a second handle to the same path.
// ---------------------------------------------------------------------------------------

/// alice holds `a = 1` (and `b = 2` when `two`), flushed; returns the next call's root.
fn audit_a2_setup(two: bool) -> Ex {
    testing::use_mock();
    let mut ex: Ex = load_root();
    deposit(&mut ex, alice(), "a", 1);
    if two {
        deposit(&mut ex, alice(), "b", 2);
    }
    flush_root(&ex);
    testing::begin_call();
    load_root()
}

fn audit_keys_below(rec: &[u8]) -> usize {
    store().keys().filter(|k| k.len() > rec.len() && k.starts_with(rec)).count()
}

/// A2, the reviewer's "lost write / no rebind panic" trace. Observed: no panic (the taken
/// handle stays `Bound`; nothing rebinds it), and NO lost write: both handles address the same
/// cache slots by path, so each sees the other's writes and the flush writes all of them.
#[test]
fn audit_a2_taken_field_handle_and_recreated_handle_stay_coherent() {
    let mut ex = audit_a2_setup(false);
    let mut taken = core::mem::take(&mut ex.accounts.get_mut(&alice()).unwrap().balances);
    assert!(taken.path().is_some(), "the escaped handle is still bound to alice's balances");
    taken.insert("b".into(), 2);
    {
        let view = ex.accounts.at_mut(&alice()).expect("record still present");
        assert_eq!(view.balances.get(&"b".into()), Some(&2), "recreated handle sees taken's write");
        view.balances.insert("c".into(), 3);
        *view.balances.get_mut(&"a".into()).unwrap() += 10;
    }
    assert_eq!(taken.get(&"c".into()), Some(&3), "taken sees the recreated handle's write");
    assert_eq!(taken.get(&"a".into()), Some(&11));
    assert_eq!(taken.len(), 3);
    // The same escape works through `at_mut` (its fields are `&mut` handles in the arena).
    let taken2 = core::mem::take(ex.accounts.at_mut(&alice()).unwrap().balances);
    assert_eq!(taken2.len(), 3);
    flush_root(&ex);

    testing::begin_call();
    let ex: Ex = load_root();
    let b = ex.accounts.at(&alice()).balances;
    assert_eq!(b.len(), 3);
    assert_eq!(
        (b.get(&"a".into()), b.get(&"b".into()), b.get(&"c".into())),
        (Some(&11), Some(&2), Some(&3)),
        "every write through either handle reached storage"
    );
}

/// A2 hole 1, fixed in API round 4: a `&V` from the escaped handle, then an overwrite of that
/// key through the recreated handle, then a read of the `&V`. Safe code, compiles (two
/// different variables). `cache::put` used to free the old box (use-after-free under both
/// Miri models); it now buries it until the flush, so the `&V` still reads the old value.
/// Runs in plain `cargo test` too; the verdict that matters is Miri's.
#[test]
fn audit_a2_ref_from_taken_handle_dangles_after_overwrite_through_recreated_handle() {
    let mut ex = audit_a2_setup(false);
    let taken = core::mem::take(&mut ex.accounts.get_mut(&alice()).unwrap().balances);
    let r: &u128 = taken.get(&"a".into()).unwrap();
    ex.accounts.at_mut(&alice()).unwrap().balances.insert("a".into(), 99);
    let observed = *r;
    assert_eq!(observed, 1, "the overwritten value lives until the flush");
    assert_eq!(taken.get(&"a".into()), Some(&99), "and a new lookup sees the new one");
    flush_root(&ex);
}

/// A2 hole 1, the same through a removal that hands nothing back (`discard`): the cached
/// value is buried, not freed, so the `&V` from the escaped handle stays valid.
#[test]
fn audit_a2_ref_from_taken_handle_survives_discard_through_recreated_handle() {
    let mut ex = audit_a2_setup(false);
    let taken = core::mem::take(&mut ex.accounts.get_mut(&alice()).unwrap().balances);
    let r: &u128 = taken.get(&"a".into()).unwrap();
    assert!(ex.accounts.at_mut(&alice()).unwrap().balances.discard(&"a".into()));
    assert_eq!(*r, 1);
    assert_eq!(taken.get(&"a".into()), None);
    flush_root(&ex);
}

fn audit_panic_message(e: Box<dyn core::any::Any + Send>) -> String {
    e.downcast_ref::<String>().cloned().or_else(|| e.downcast_ref::<&str>().map(|m| m.to_string())).unwrap_or_default()
}

/// A2 hole 2, fixed in API round 4: `remove` clears alice's children and deletes the record;
/// the escaped handle (not detached, only the returned record is) used to write freely,
/// leaving 4 orphan keys under the removed path that a record later created at the same key
/// inherited. Its write now panics (the removal bumped the epoch past the handle's stamp), so
/// nothing is orphaned and a recreated alice starts empty.
#[test]
fn audit_a2_taken_handle_writes_after_parent_remove_orphan_and_resurrect() {
    let mut ex = audit_a2_setup(false);
    let mut taken = core::mem::take(&mut ex.accounts.get_mut(&alice()).unwrap().balances);
    let removed = ex.accounts.remove(&alice()).unwrap();
    assert_eq!(format!("{:?}", removed.balances), "Map(Detached)", "the left-behind handle is the one detached");
    assert!(taken.path().is_some(), "the escaped handle is still bound");
    let e = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _ = taken.insert("z".into(), 9);
    }))
    .expect_err("a write through the escaped handle after the parent's remove");
    assert!(audit_panic_message(e).contains("was removed or started `remove_step` clearing"));
    // Every other write path of the handle is refused too.
    for op in 0..4 {
        let e = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| match op {
            0 => drop(taken.get_mut(&"a".into())),
            1 => drop(taken.entry("y".into()).or_insert(1)),
            2 => drop(taken.remove(&"a".into())),
            _ => taken.clear(),
        }))
        .expect_err("every write path checks");
        assert!(audit_panic_message(e).contains("was removed"), "op {op}");
    }
    assert_eq!(taken.get(&"a".into()), None, "reads are not refused; the entry is gone");
    flush_root(&ex);
    let rec = alice_record_key();
    assert!(!store().contains_key(&rec), "the record is gone");
    assert_eq!(audit_keys_below(&rec), 0, "no orphans");

    testing::begin_call();
    let mut ex: Ex = load_root();
    ex.accounts.try_insert(alice(), Account::default()).expect("alice was removed");
    flush_root(&ex);
    testing::begin_call();
    let ex: Ex = load_root();
    let b = ex.accounts.at(&alice()).balances;
    assert_eq!((b.len(), b.get(&"z".into())), (0, None), "the new record inherits nothing");
}

/// A2 hole 3, fixed in API round 4: after `remove_step(&alice, 3)` flags the record (3 host
/// removes: the move to the flag key, then 2 left, too few for an entry), `at_mut` /
/// `get_mut` / `entry` on it panic (pinned by `writes_below_a_clearing_record_panic_in_the_same_call`
/// in state_v2_exchange), and now so does the escaped handle: `set_clearing` bumps the epoch
/// like a removal. The later `remove_step` calls clear everything and leave nothing below.
#[test]
fn audit_a2_taken_handle_bypasses_remove_some_clearing_guard() {
    let mut ex = audit_a2_setup(true);
    let mut taken = core::mem::take(&mut ex.accounts.get_mut(&alice()).unwrap().balances);
    assert_eq!(ex.accounts.remove_step(&alice(), nz(3)), Removal::Pending, "flagged, nothing cleared");
    let e = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _ = taken.insert("late".into(), 5);
    }))
    .expect_err("a write below a clearing record through the escaped handle");
    assert!(audit_panic_message(e).contains("was removed or started `remove_step` clearing"));
    assert_eq!(taken.get(&"a".into()), Some(&1), "reads of a clearing record are still allowed");
    flush_root(&ex);
    let rec = alice_record_key();
    let flag = super::key::flag_key_bytes(&rec);
    assert!(!store().contains_key(&rec) && store().contains_key(&flag), "D9: moved to its flag key");
    assert_eq!(audit_keys_below(&rec), 2 * 3 + 1 + 1, "a and b, the header, the flag key; no `late`");

    let mut calls = 0;
    loop {
        testing::begin_call();
        let mut ex: Ex = load_root();
        let more = ex.accounts.remove_step(&alice(), nz(8)) == Removal::Pending;
        flush_root(&ex);
        calls += 1;
        if !more {
            break;
        }
    }
    assert_eq!(calls, 1, "two entries (6), the header and the record (flag key): budget 8");
    assert!(!store().contains_key(&rec));
    assert_eq!(audit_keys_below(&rec), 0, "nothing left below");
}

/// A2(b) must not refuse the legitimate sequence the generation check exists to tell apart
/// from an escaped handle: remove a record, re-insert one at the same key, write below it,
/// all in one call. The new record's handles are bound after the removal.
#[test]
fn audit_a2_remove_then_reinsert_then_write_below_in_one_call() {
    let mut ex = audit_a2_setup(true);
    let _ = ex.accounts.remove(&alice()).unwrap();
    let a = ex.accounts.try_insert(alice(), Account::default()).expect("removed above");
    assert!(a.balances.insert("n".into(), 7), "new key");
    ex.accounts.get_mut(&alice()).unwrap().balances.insert("m".into(), 8);
    ex.accounts.at_mut(&alice()).unwrap().balances.insert("o".into(), 9);
    // And after `remove_step` finishing in the same call, through `entry().or_default()`.
    assert_eq!(ex.accounts.remove_step(&alice(), nz(10)), Removal::Done);
    ex.accounts.entry(alice()).or_default().balances.insert("p".into(), 10);
    flush_root(&ex);
    testing::begin_call();
    let ex: Ex = load_root();
    let b = ex.accounts.at(&alice()).balances;
    assert_eq!((b.len(), b.get(&"p".into()), b.get(&"a".into())), (1, Some(&10), None));
}

/// A2(b): a removal elsewhere does not refuse a handle that is not below it (the slow path
/// scans, finds nothing, and moves the handle's stamp forward), including an escaped one.
#[test]
fn audit_a2_unrelated_removal_does_not_refuse_other_handles() {
    let mut ex = audit_a2_setup(false);
    deposit(&mut ex, bob(), "a", 5);
    let mut taken = core::mem::take(&mut ex.accounts.get_mut(&alice()).unwrap().balances);
    let _ = ex.accounts.remove(&bob()).unwrap();
    assert!(taken.insert("b".into(), 2), "alice's balances are not below bob");
    assert!(ex.accounts.at_mut(&alice()).unwrap().balances.insert("c".into(), 3));
    flush_root(&ex);
    testing::begin_call();
    let ex: Ex = load_root();
    assert_eq!(ex.accounts.at(&alice()).balances.len(), 3);
    assert!(!ex.accounts.contains_key(&bob()));
}

// ---------------------------------------------------------------------------------------
// Review round (slim-b, Oct 22): a value's decoder is user code (a hand-written `Value`, or a
// Borsh `deserialize_reader`) and may call back into the cache, through another handle, for
// the very key being decoded. `get` must not hold a `&mut` to the slot across that call: the
// inner operation's own `&mut` to the slot invalidates it under Stacked Borrows. Miri verdict
// as for the audit tests above.
// ---------------------------------------------------------------------------------------

const REENTRANT_TAG: u32 = 43;

std::thread_local! {
    static REENTRANT_SAW: core::cell::Cell<Option<bool>> = const { core::cell::Cell::new(None) };
}

/// A leaf whose decoder asks the map it is stored in whether its own key exists, then reads
/// the rest of its bytes (still the slot's, after the re-entry).
#[derive(borsh::BorshSerialize)]
struct Reentrant(u32, u8);
impl BorshLeaf for Reentrant {}
impl borsh::BorshDeserialize for Reentrant {
    fn deserialize_reader<R: borsh::io::Read>(r: &mut R) -> borsh::io::Result<Self> {
        let key = u32::deserialize_reader(r)?;
        let m: LookupMap<u32, Reentrant> = at(REENTRANT_TAG);
        REENTRANT_SAW.with(|c| c.set(Some(m.contains_key(&key))));
        Ok(Reentrant(key, u8::deserialize_reader(r)?))
    }
}

/// `get(k)` whose decoder calls `contains_key(k)` through a second handle. Under Miri Stacked
/// Borrows this was UB at f683e42e: `get` held `&mut Slot` across the decoder and wrote the
/// decoded value through it after `contains_key` had retagged the same slot.
#[test]
fn review_decoder_reenters_the_cache_for_its_own_key() {
    testing::use_mock();
    let mut m: LookupMap<u32, Reentrant> = at(REENTRANT_TAG);
    m.insert(5, Reentrant(5, 9));
    end_call();
    REENTRANT_SAW.with(|c| c.set(None));
    assert_eq!(m.get(&5).map(|v| (v.0, v.1)), Some((5, 9)));
    assert_eq!(REENTRANT_SAW.with(core::cell::Cell::get), Some(true), "the decoder saw its key");
    assert_eq!(m.get(&5).map(|v| (v.0, v.1)), Some((5, 9)), "decoded once, then served from the cache");
    let r = end_call();
    assert_eq!((r.reads(), r.has_keys(), r.writes(), r.removes()), (1, 0, 0, 0));
}
