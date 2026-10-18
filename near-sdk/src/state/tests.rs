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
use std::vec::Vec as StdVec;

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
    fn __view_mut<'a>(path: Path) -> AccountAtMut<'a> {
        AccountAtMut {
            nonce: <u64 as ViewFieldMut<'a, Account>>::__view_field_mut(path.field(1)),
            balances: <Map<String, u128> as ViewFieldMut<'a, Account>>::__view_field_mut(
                path.field(2),
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
        let mut w = p::RecordWriter::new();
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
    m.insert(1, vec![1]); // the entry under test, slot 0
    m.insert(2, neighbor); // the neighbor, last slot: it is the key that moves on removal
    end_call();
    m
}

#[test]
fn itermap_insert_and_update_costs() {
    let mut m = iter_fixture(big());
    m.insert(3, vec![3]);
    let r = end_call();
    assert_eq!(r.rwd(), (2, 4, 0), "insert new: index read, header read, 4 writes");

    m.insert(3, vec![4]);
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
        assert!(r.max_value_bytes() <= 8, "no host call moves the neighbor's value");
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
    assert_eq!(r.rwd(), (3, 0, 4), "emptying the map removes the header instead of writing 0");
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
        m.insert(i, i as u64);
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
        m.insert(i, 0);
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
        m.insert(i, 0);
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
        m.insert(i, 0);
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
    m.insert(1_000_000, 1);
    m.insert(1_000_001, 1);
    end_call();
    m.insert(7, 7);
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
            m.insert(k, x);
        } else {
            m.discard(&k);
        }
    }
    m.insert(1_000_000, 1);
    m.insert(1_000_001, 1);
    end_call();
    m.insert(7_000_000, 7);
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
    m.insert("usdt".into(), 0);
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
    ex.accounts.insert_new(alice(), Account::default());
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
        a.balances.insert(t.into(), 100);
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
    assert_eq!(f, vec![(1, &borsh::to_vec(&2u64).unwrap()[..]), (9, &[0xAA, 0xBB][..])]);
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
        record::parse(&store()[key.as_bytes()]).unwrap().into_iter().map(|(t, _)| t).collect::<StdVec<_>>()
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
    ex2.accounts.insert_new(alice(), Account::default());
    flush_root(&ex2);
    let r = receipt();
    assert_eq!(r.has_keys() - before.has_keys(), 1);
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
    seed(&alice_record_key(), &[0x00, 0x00]);
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
    let f = record::parse(&store()[&alice_record_key()]).unwrap().into_iter().map(|(t, _)| t).collect::<StdVec<_>>();
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
    // root read, account read, usdt entry read, header read; account record write, 4 writes
    assert_eq!(r.rwd(), (4, 6, 0), "the 6th write is the root itself: absent before the first call, so it is created");

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

/// D6: `insert_new` returns the record bound to its key, so its collections are usable at
/// once; on `Map` it costs what a new-key `insert` costs.
#[test]
fn insert_new_returns_the_bound_record() {
    testing::use_mock();
    let mut m: Map<u32, Account> = at(11);
    let a = m.insert_new(1, Account { nonce: 3, ..Default::default() });
    a.balances.insert("usdt".into(), 5);
    a.nonce += 1;
    let r = end_call();
    assert_eq!(r.rwd(), (4, 8, 0), "outer: index + header reads, 4 writes; balances: the same");
    assert_eq!(m.get(&1).unwrap().nonce, 4);
    assert_eq!(m.at(&1).balances.get(&"usdt".to_string()), Some(&5));
    let mut lm: LookupMap<u32, Account> = at(12);
    lm.insert_new(1, Account::default()).nonce = 2;
    let r = end_call();
    assert_eq!((r.has_keys(), r.writes()), (1, 1), "LookupMap: 1 existence check, 1 write");
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

/// Item 15 / D6: `insert_new` over an existing key panics instead of silently inheriting the
/// old record's children (round 1 behaviour). Plain `insert` does not compile for this record.
#[test]
#[should_panic(expected = "record exists; use get_mut, or remove it first")]
fn insert_new_over_existing_record_panics() {
    testing::use_mock();
    let mut ex: Ex = load_root();
    deposit(&mut ex, alice(), "a", 1);
    flush_root(&ex);
    testing::begin_call();
    let mut ex: Ex = load_root();
    ex.accounts.insert_new(alice(), Account { nonce: 9, balances: Map::default() });
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
    ex.accounts.insert_new(bob(), a);
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
    new.balances.insert("a".into(), bal);
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
    ex.accounts.insert_new(bob(), taken);
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
        m.insert(i, i);
    }
    end_call();
    assert!(m.clear_some(4));
    let r = end_call();
    assert_eq!(r.rwd(), (1 + 4, 1, 12), "header + 4 slot reads; header write; 3 removes each");
    assert!(m.clear_some(4));
    end_call();
    assert!(!m.clear_some(4));
    end_call();
    assert_eq!(store().len(), 0);
}

#[test]
fn grouped_map_groups_are_independent_itermaps() {
    testing::use_mock();
    let mut g: GroupedMap<u32, u64, u64> = at(4);
    g.group_mut(&1).insert(10, 1);
    g.group_mut(&1).insert(11, 1);
    g.group_mut(&2).insert(10, 2);
    end_call();
    assert_eq!(g.group(&1).len(), 2);
    assert_eq!(g.group(&2).get(&10), Some(&2));
    end_call();
    g.group_mut(&2).remove(&10);
    let r = end_call();
    assert_eq!(r.rwd(), (3, 0, 4), "last entry of a group: header removed when the group empties");
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
        m.insert(i, vec![0; 1024]);
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
        m.insert(i, i);
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
        a.insert(i, ());
        b.insert(i, ());
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
fn at_mut_writes_below_the_record_without_reading_it() {
    testing::use_mock();
    let mut m: Map<u32, Account> = at(8);
    m.entry(1).or_default().nonce = 7;
    end_call();
    m.at_mut(&1).unwrap().balances.insert("usdt".into(), 5);
    let r = end_call();
    assert_eq!(r.has_keys(), 1, "existence check");
    assert_eq!(r.rwd(), (2, 4, 0), "balances index + header reads, 4 writes; record not read");
    assert_eq!(m.get(&1).unwrap().balances.get(&"usdt".to_string()), Some(&5));
    assert_eq!(m.get(&1).unwrap().nonce, 7);
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
    assert_eq!(r.has_keys(), 2);
    assert_eq!(r.keys_written().len(), 0, "nothing created");
}
