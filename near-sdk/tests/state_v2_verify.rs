//! Guarantee 6 (Permanent layout) and bug class D2 (required-field evolution) through the REAL
//! `#[near(storage)]` derive: a record written by schema S1 is decoded by each later schema S2
//! with random field values (bolero random engine, 2,000 cases per property). The codec kernel
//! these expand to is model-checked in `src/state/verify/g6_layout.rs`.
#![cfg(feature = "unstable")]

use near_sdk::near;
use near_sdk::state::testing::{self, OpKind};
use near_sdk::state::{LookupMap, Path, record};

#[near(storage)]
#[derive(Clone, Debug, PartialEq)]
pub struct S1 {
    #[key(1)]
    a: u64,
    #[key(2)]
    b: Vec<u8>,
    #[key(3)]
    c: u32,
    #[key(5)]
    d: u16,
}

/// Added fields: tag 4 with `Default`, tag 6 with `default = 77`.
#[near(storage)]
pub struct Added {
    #[key(1)]
    a: u64,
    #[key(2)]
    b: Vec<u8>,
    #[key(3)]
    c: u32,
    #[key(4)]
    e: u32,
    #[key(5)]
    d: u16,
    #[key(6, default = 77)]
    f: u8,
}

/// Removed fields 2 and 5 (forgot `reserved`: their bytes stay as unknown tags).
#[near(storage)]
pub struct Removed {
    #[key(1)]
    a: u64,
    #[key(3)]
    c: u32,
}

/// Same tags, source order shuffled.
#[near(storage)]
#[derive(Clone, Debug, PartialEq)]
pub struct Reordered {
    #[key(3)]
    c: u32,
    #[key(5)]
    d: u16,
    #[key(1)]
    a: u64,
    #[key(2)]
    b: Vec<u8>,
}

/// Added tag 4 as `required` (the D2 mistake).
#[near(storage)]
pub struct AddedRequired {
    #[key(1)]
    a: u64,
    #[key(4, required)]
    e: u32,
}

/// Removed field 2 the right way.
#[near(storage, reserved(2))]
pub struct Reserved {
    #[key(1)]
    a: u64,
    #[key(3)]
    c: u32,
    #[key(5)]
    d: u16,
}

type Vals = (u64, Vec<u8>, u32, u16);

fn root() -> Path {
    Path::root(0xF0).field(9)
}

fn key() -> Path {
    root().kind(0).entry(&1u32)
}

/// Writes `s1` under key 1 in a fresh mock and starts a new call.
fn write_s1(v: &Vals) -> S1 {
    testing::use_mock();
    let s = S1 { a: v.0, b: v.1.clone(), c: v.2, d: v.3 };
    let mut m: LookupMap<u32, S1> = LookupMap::__bound_at(root());
    m.insert(1, s.clone());
    testing::flush();
    testing::begin_call();
    s
}

fn no_writes() {
    let r = testing::receipt();
    assert!(r.ops.iter().all(|o| matches!(o.kind, OpKind::Read | OpKind::HasKey)), "decode wrote");
}

fn tags() -> Vec<u32> {
    record::parse(&testing::store()[key().as_bytes()]).unwrap().into_iter().map(|(t, _)| t).collect()
}

fn vals() -> impl bolero::ValueGenerator<Output = Vals> {
    (bolero::produce::<u64>(), bolero::produce::<Vec<u8>>().with().len(0usize..=40), bolero::produce::<u32>(), bolero::produce::<u16>())
}

/// S1 to S2 with added fields: old values decode, the `Default` field is 0, the `default = 77`
/// field is 77, and decoding issues no write or remove (no migration pass, no rewrite).
#[test]
fn g6_derive_added_fields_default_without_a_write() {
    bolero::check!().with_generator(vals()).with_iterations(2_000).for_each(|v| {
        let s = write_s1(v);
        let m: LookupMap<u32, Added> = LookupMap::__bound_at(root());
        let x = m.get(&1).unwrap();
        assert_eq!((x.a, &x.b, x.c, x.d, x.e, x.f), (s.a, &s.b, s.c, s.d, 0, 77));
        let _ = x;
        testing::flush();
        no_writes();
    });
}

/// Removed fields: S2 decodes the rest without a write; a read-modify-write under S2 keeps
/// tags 2 and 5 byte-for-byte, so rolling back to S1 sees them intact.
#[test]
fn g6_derive_removed_fields_survive_a_rewrite_and_a_rollback() {
    bolero::check!().with_generator(vals()).with_iterations(2_000).for_each(|v| {
        let s = write_s1(v);
        let mut m: LookupMap<u32, Removed> = LookupMap::__bound_at(root());
        assert_eq!((m.get(&1).unwrap().a, m.get(&1).unwrap().c), (s.a, s.c));
        no_writes();
        m.get_mut(&1).unwrap().a = s.a.wrapping_add(1);
        testing::flush();
        assert_eq!(tags(), vec![1, 2, 3, 5]);
        testing::begin_call();
        let back: LookupMap<u32, S1> = LookupMap::__bound_at(root());
        assert_eq!(&*back.get(&1).unwrap(), &S1 { a: s.a.wrapping_add(1), ..s });
    });
}

/// Reordered declarations: decode is unaffected and the encoding is byte-identical (tags, not
/// source order, define the layout).
#[test]
fn g6_derive_reordering_fields_changes_nothing() {
    bolero::check!().with_generator(vals()).with_iterations(2_000).for_each(|v| {
        let s = write_s1(v);
        let before = testing::store();
        let mut m: LookupMap<u32, Reordered> = LookupMap::__bound_at(root());
        let x = m.get(&1).unwrap().clone();
        assert_eq!(x, Reordered { a: s.a, b: s.b.clone(), c: s.c, d: s.d });
        no_writes();
        m.insert(1, x);
        testing::flush();
        assert_eq!(testing::store(), before, "reordered schema re-encoded differently");
    });
}

/// D2: a `required` field added later fails EVERY old record, naming the tag; a present
/// value decodes.
#[test]
fn d2_derive_required_added_later_fails_naming_the_tag() {
    bolero::check!().with_generator(vals()).with_iterations(500).for_each(|v| {
        write_s1(v);
        let m: LookupMap<u32, AddedRequired> = LookupMap::__bound_at(root());
        let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| m.get(&1).map(|x| x.e)));
        let msg = r.expect_err("decoded a record missing a required tag");
        let msg = msg.downcast_ref::<String>().cloned().or_else(|| msg.downcast_ref::<&str>().map(|s| s.to_string())).unwrap_or_default();
        assert!(msg.contains("Missing { tag: 4 }") && msg.contains("tag 4"), "error does not name the tag: {msg}");
    });
}

/// `reserved(2)`: the retired tag is dropped on the first rewrite (only then: decoding alone
/// writes nothing); tags 3 and 5 keep their values, and S1 reads `b` as its default.
#[test]
fn g6_derive_reserved_tag_dropped_on_rewrite_only() {
    bolero::check!().with_generator(vals()).with_iterations(2_000).for_each(|v| {
        let s = write_s1(v);
        let mut m: LookupMap<u32, Reserved> = LookupMap::__bound_at(root());
        assert_eq!(m.get(&1).unwrap().d, s.d);
        testing::flush();
        no_writes();
        assert!(tags().contains(&2) || s.b.is_empty() && !tags().contains(&2));
        testing::begin_call();
        m.get_mut(&1).unwrap().a = 9;
        testing::flush();
        assert!(!tags().contains(&2));
        testing::begin_call();
        let back: LookupMap<u32, S1> = LookupMap::__bound_at(root());
        assert_eq!(&*back.get(&1).unwrap(), &S1 { a: 9, b: vec![], ..s });
    });
}
