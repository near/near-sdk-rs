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
    record::parse(&testing::store()[key().as_bytes()]).unwrap().into_iter().map(|(t, _, _)| t).collect()
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
        // Refresh the mocked context before its gas runs out: the expected decode panic goes
        // through a mocked host call that charges gas, and after ~9.7k inputs (libFuzzer round,
        // Oct 18) the accumulated gas turned it into `GasLimitExceeded`, which does not name the tag.
        if near_sdk::env::used_gas() > near_sdk::Gas::from_tgas(200) {
            near_sdk::testing_env!(near_sdk::test_utils::VMContextBuilder::new().build());
        }
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

// ---------------------------------------------------------------------------------------
// C7 / M4: the migrate-time v1-prefix guard, wired (harnesses: src/state/verify/c7_m4_alias.rs)
// ---------------------------------------------------------------------------------------

mod m4 {
    use near_sdk::state::testing;
    use near_sdk::state::{ContractState, Map, Migrating};
    use near_sdk::store::LookupMap as V1LookupMap;
    use near_sdk::test_utils::VMContextBuilder;
    use near_sdk::{env, near, testing_env};

    #[near(contract_state(layout = v2))]
    pub struct C {
        #[key(1, required)]
        pub m: Migrating<V1LookupMap<[u8; 1], u8>, Map<u8, u8>>,
        #[key(2)]
        pub n: u64,
    }

    fn fresh() {
        testing_env!(VMContextBuilder::new().build());
        testing::use_env_host();
        testing::begin_call();
    }

    fn msg(e: Box<dyn std::any::Any + Send>) -> String {
        e.downcast_ref::<String>().cloned().or_else(|| e.downcast_ref::<&str>().map(|s| s.to_string())).unwrap_or_default()
    }

    /// An empty v1 prefix or one starting with the root byte is rejected when the `Migrating`
    /// field is written; a safe prefix (`b"s"`) is accepted and round-trips.
    #[test]
    fn c7_migrating_rejects_aliasing_v1_prefixes_on_write() {
        for (prefix, ok) in [(vec![], false), (vec![0xF0], false), (vec![0xF0, 1], false), (b"s".to_vec(), true), (vec![0xF1], true)] {
            fresh();
            let r = std::panic::catch_unwind(|| {
                let c = C { m: Migrating::new(V1LookupMap::new(prefix.clone())), n: 1 };
                c.state_write();
            });
            match (ok, r) {
                (true, Ok(())) => {
                    testing::begin_call();
                    assert_eq!(C::state_read().unwrap().n, 1);
                }
                (false, Err(e)) => {
                    assert!(msg(e).contains("can alias v2 keys"), "prefix {prefix:02x?}");
                    assert!(!env::storage_has_key(&[0xF0]), "a rejected migrate wrote the root");
                }
                (true, Err(e)) => panic!("safe prefix {prefix:02x?} rejected: {}", msg(e)),
                (false, Ok(())) => panic!("aliasing prefix {prefix:02x?} accepted"),
            }
        }
    }

    /// A stored record whose `Migrating` v1 half has an empty prefix (written before the guard
    /// existed) fails to LOAD, naming the reason, instead of silently aliasing.
    #[test]
    fn c7_migrating_rejects_an_aliasing_v1_prefix_on_load() {
        fresh();
        let mut w = near_sdk::state::record::RecordWriter::new();
        w.push(1, near_sdk::borsh::to_vec(&V1LookupMap::<[u8; 1], u8>::new(vec![])).unwrap());
        env::storage_write(&[0xF0], &w.finish(None, |_| true));
        let r = std::panic::catch_unwind(|| C::state_read().map(|c| c.n));
        assert!(msg(r.expect_err("loaded an aliasing layout")).contains("can alias v2 keys"));
    }

    /// A v1 map kept beside the v2 root without `Migrating`. Since T7 (API round 3) that is
    /// only possible as a `#[key(n, legacy)]` root field (a bare v1 type at the root is a
    /// derive error), so the fixture uses one.
    mod unwrapped {
        use super::*;
        #[near(contract_state(layout = v2))]
        pub struct L {
            #[key(1, legacy)]
            pub bare: V1LookupMap<[u8; 1], u8>,
            #[key(2)]
            pub n: u64,
        }
    }
    use unwrapped::L;

    /// The stated exception, pinned as a hazard: a v1 map NOT wrapped in `Migrating` (a
    /// `#[key(n, legacy)]` field kept beside the v2 root) is not checked: `legacy` stores its
    /// Borsh bytes and nothing runs `check_v1_prefix` on them. With an empty prefix and a
    /// `[u8; 1]` key, a caller-chosen key `[0xF0]` overwrites the v2 root record.
    #[test]
    fn hazard_an_unwrapped_empty_prefix_v1_map_overwrites_the_v2_root() {
        fresh();
        let mut l = L { bare: V1LookupMap::new(vec![]), n: 7 };
        l.state_write();
        assert_eq!(L::state_read().map(|l| l.n), Some(7), "the legacy field with an empty prefix loads unchecked");
        l.bare.set([0xF0], Some(0)); // `set` writes without reading the old value
        l.bare.flush();
        drop(l);
        assert_eq!(env::storage_read(&[0xF0]).unwrap(), vec![0], "HAZARD: the v2 root was overwritten by a v1 entry");
        testing::begin_call();
        assert!(std::panic::catch_unwind(|| L::state_read().map(|l| l.n)).is_err(), "the contract no longer loads");
    }

    mod composite {
        use super::*;
        use near_sdk::collections::{TreeMap, UnorderedMap};
        use near_sdk::store::IterableMap;
        use near_sdk::store::key::Identity;

        #[near(contract_state(layout = v2))]
        pub struct T {
            #[key(1, required)]
            pub a: Migrating<TreeMap<u8, u8>, Map<u8, u8>>,
            #[key(2, required)]
            pub b: Migrating<UnorderedMap<u8, u8>, Map<u8, u8>>,
            #[key(3, required)]
            pub c: Migrating<IterableMap<u8, u8, Identity>, Map<u8, u8>>,
            #[key(4, required)]
            pub d: Migrating<near_sdk::collections::LookupMap<u8, u8>, Map<u8, u8>>,
        }

        fn build(pa: &[u8], pb: &[u8], pc: &[u8]) -> T {
            T {
                a: Migrating::new(TreeMap::new(pa.to_vec())),
                b: Migrating::new(UnorderedMap::new(pb.to_vec())),
                c: Migrating::new(IterableMap::with_hasher(pc.to_vec())),
                d: Migrating::new(near_sdk::collections::LookupMap::new(b"d".to_vec())),
            }
        }

        /// The guard covers the iterable v1 maps (every derived prefix: `i`/`k`/`v`, `v`/`n`,
        /// `v`/`m`): a root-byte prefix on any one of them is rejected without a write; an
        /// EMPTY user prefix is accepted, correctly, because each derived prefix then starts
        /// with an ASCII letter, not the root byte.
        #[test]
        fn c7_guard_covers_tree_unordered_and_iterable_maps() {
            for (i, (pa, pb, pc)) in [(&[0xF0][..], &b"b"[..], &b"c"[..]), (b"a", &[0xF0], b"c"), (b"a", b"b", &[0xF0])].into_iter().enumerate() {
                fresh();
                let r = std::panic::catch_unwind(|| build(pa, pb, pc).state_write());
                assert!(msg(r.expect_err("accepted")).contains("can alias v2 keys"), "case {i}");
                assert!(!env::storage_has_key(&[0xF0]));
            }
            fresh();
            build(&[], &[], &[]).state_write();
            testing::begin_call();
            assert!(T::state_read().is_some());
            // collections::LookupMap: an empty prefix and a root-byte prefix are rejected
            for bad in [vec![], vec![0xF0, 7]] {
                fresh();
                let r = std::panic::catch_unwind(|| {
                    let mut t = build(b"a", b"b", b"c");
                    t.d = Migrating::new(near_sdk::collections::LookupMap::new(bad.clone()));
                    t.state_write()
                });
                assert!(msg(r.expect_err("collections::LookupMap prefix accepted")).contains("can alias v2 keys"), "{bad:02x?}");
            }
        }
    }
}

// ---------------------------------------------------------------------------------------
// A3: rollback subtree ownership. API round 5: removal fails closed on unknown collection tags.
// ---------------------------------------------------------------------------------------

mod a3 {
    use near_sdk::near;
    use near_sdk::state::testing;
    use near_sdk::state::{LookupMap, Map, Path, record};
    use std::panic::{AssertUnwindSafe, catch_unwind};

    /// Version B: the record owns a child map at tag 2.
    #[near(storage)]
    #[derive(Default)]
    pub struct RecB {
        #[key(1)]
        n: u64,
        #[key(2)]
        kids: Map<u8, u64>,
    }

    /// Version A (the rollback target): no child map; tag 2 is unknown to it.
    #[near(storage)]
    #[derive(Default)]
    pub struct RecA {
        #[key(1)]
        n: u64,
    }

    /// Version A', which lists B's collection tag in `reserved(2)`: known-and-empty.
    #[near(storage, reserved(2))]
    #[derive(Default)]
    pub struct RecAReserved {
        #[key(1)]
        n: u64,
    }

    /// Version C: A plus a scalar at tag 3 (an unknown scalar tag to A).
    #[near(storage)]
    #[derive(Default)]
    pub struct RecC {
        #[key(1)]
        n: u64,
        #[key(3)]
        note: u32,
    }

    /// Version D: A plus two scalars whose bytes look like markers. Unknown to A.
    #[near(storage)]
    #[derive(Default)]
    pub struct RecD {
        #[key(1)]
        n: u64,
        #[key(2)]
        small: u8,
        #[key(3)]
        shaped: [u8; 3],
    }

    /// A nested record that owns a collection, and its rollback shape.
    #[near(storage)]
    #[derive(Default)]
    pub struct InnerB {
        #[key(1)]
        x: u8,
        #[key(2)]
        kids: Map<u8, u64>,
    }
    #[near(storage)]
    #[derive(Default)]
    pub struct InnerA {
        #[key(1)]
        x: u8,
    }
    #[near(storage)]
    #[derive(Default)]
    pub struct OuterB {
        #[key(1)]
        n: u64,
        #[key(2)]
        inner: InnerB,
    }
    /// Knows `inner` but as `InnerA`, which does not declare `kids`.
    #[near(storage)]
    #[derive(Default)]
    pub struct OuterKnowsInnerA {
        #[key(1)]
        n: u64,
        #[key(2)]
        inner: InnerA,
    }
    /// Does not declare `inner` at all.
    #[near(storage)]
    #[derive(Default)]
    pub struct OuterA {
        #[key(1)]
        n: u64,
    }

    fn root() -> Path {
        Path::root(0xF0).field(9)
    }

    fn msg(e: Box<dyn std::any::Any + Send>) -> String {
        e.downcast_ref::<String>().cloned().or_else(|| e.downcast_ref::<&str>().map(|s| s.to_string())).unwrap_or_default()
    }

    fn quiet<R>(f: impl FnOnce() -> R) -> std::thread::Result<R> {
        let hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(|_| {}));
        let r = catch_unwind(AssertUnwindSafe(f));
        std::panic::set_hook(hook);
        r
    }

    fn kid_key() -> Vec<u8> {
        root().kind(0).entry(&1u32).field(2).kind(0).entry(&5u8).as_bytes().to_vec()
    }

    /// Version B writes record 1 with one child; the store afterwards.
    fn write_b() {
        testing::use_mock();
        let mut b: Map<u32, RecB> = Map::__bound_at(root());
        b.try_insert(1, RecB { n: 1, ..Default::default() }).unwrap();
        b.at_mut(&1).unwrap().kids.insert(5, 50);
        testing::flush();
    }

    /// Runs `f` in a fresh call, expects the A3 panic, and checks that it names tag `tag`,
    /// the collection type and both fixes, and that storage is unchanged.
    fn assert_refused(what: &str, tag: &str, f: impl FnOnce()) {
        testing::begin_call();
        let before = testing::store();
        let e = quiet(|| {
            f();
            testing::flush();
        })
        .expect_err(what);
        let m = msg(e);
        assert!(m.contains(&format!("carry tag {tag}, a `Map` field")), "{what}: {m}");
        assert!(m.contains("Roll forward") && m.contains("reserved("), "{what}: {m}");
        testing::begin_call();
        assert_eq!(testing::store(), before, "{what}: the refused removal changed storage");
        assert!(testing::store().contains_key(&kid_key()), "{what}");
    }

    /// The A3 pin, flipped. Version B gives record 1 a child map (tag 2, stored with its kind
    /// marker `09 01 81`, wire 1); the contract rolls back to version A, which does not declare tag 2.
    /// Every way A can remove the record (`remove`, `discard`, `remove_step`, a `LookupMap`
    /// `remove` / `replace` / `discard`, a bulk `clear` of the map) panics naming tag 2 and
    /// changes nothing, so A can never recreate record 1 over B's children. (Before round 5 A's
    /// `remove` succeeded, left `kids[5]` behind, and a redeployed B showed it under the new
    /// record.)
    #[test]
    fn a3_rollback_removal_of_a_record_with_an_unknown_collection_tag_panics() {
        write_b();
        let rec_key = root().kind(0).entry(&1u32).as_bytes().to_vec();
        let stored = testing::store()[&rec_key].clone();
        assert_eq!(
            record::parse(&stored).unwrap(),
            vec![(1, record::Wire::Scalar, &1u64.to_le_bytes()[..]), (2, record::Wire::Marker, &[record::marker::MAP][..])],
            "B's record carries the kids marker"
        );
        let nz = std::num::NonZeroU32::new(3).unwrap();
        assert_refused("remove", "2", || drop(Map::<u32, RecA>::__bound_at(root()).remove(&1)));
        assert_refused("discard", "2", || {
            let _ = Map::<u32, RecA>::__bound_at(root()).discard(&1);
        });
        assert_refused("remove_step", "2", || {
            let _ = Map::<u32, RecA>::__bound_at(root()).remove_step(&1, nz);
        });
        assert_refused("clear", "2", || Map::<u32, RecA>::__bound_at(root()).clear());
        // the same record bytes under a LookupMap of A at another path
        let lroot = Path::root(0xF0).field(10);
        let lkey = lroot.kind(0).entry(&1u32).as_bytes().to_vec();
        testing::begin_call();
        testing::seed(&lkey, &stored);
        testing::flush();
        assert_refused("LookupMap::remove", "2", || drop(LookupMap::<u32, RecA>::__bound_at(lroot.clone()).remove(&1)));
        assert_refused("LookupMap::discard", "2", || LookupMap::<u32, RecA>::__bound_at(lroot.clone()).discard(&1));
        assert_refused("LookupMap::replace", "2", || {
            drop(LookupMap::<u32, RecA>::__bound_at(lroot.clone()).replace(1, RecA { n: 2 }))
        });
        // rolling forward works: B removes its own record and its child
        testing::begin_call();
        let mut b: Map<u32, RecB> = Map::__bound_at(root());
        assert_eq!(b.remove(&1).map(|r| r.n), Some(1));
        testing::flush();
        assert!(testing::store().keys().all(|k| !k.starts_with(&rec_key)), "B's remove left keys");
    }

    /// Fails closed, not open: a record whose collection tags are all known removes as before,
    /// and so does a record carrying an unknown SCALAR tag (C wrote tag 3 = u32; A drops it on
    /// removal, as unknown scalars always were dropped with their record).
    #[test]
    fn a3_known_collections_and_unknown_scalars_still_remove() {
        write_b();
        testing::begin_call();
        let mut b: Map<u32, RecB> = Map::__bound_at(root());
        b.try_insert(2, RecB { n: 2, ..Default::default() }).unwrap();
        b.at_mut(&2).unwrap().kids.insert(1, 1);
        testing::flush();
        testing::begin_call();
        let mut b: Map<u32, RecB> = Map::__bound_at(root());
        assert!(b.remove(&1).is_some());
        assert!(b.discard(&2));
        testing::flush();
        assert!(testing::store().keys().all(|k| !k.starts_with(root().kind(0).as_bytes())), "known collections cleared");

        // C writes record 3 with an unknown-to-A scalar at tag 3; A removes it (all three paths)
        for how in 0..3 {
            testing::begin_call();
            let mut c: Map<u32, RecC> = Map::__bound_at(root());
            c.insert(3, RecC { n: 3, note: 0x0102_0304 });
            testing::flush();
            testing::begin_call();
            let mut a: Map<u32, RecA> = Map::__bound_at(root());
            match how {
                0 => assert_eq!(a.remove(&3).map(|r| r.n), Some(3)),
                1 => assert!(a.discard(&3)),
                _ => assert_eq!(a.remove_step(&3, std::num::NonZeroU32::new(3).unwrap()), near_sdk::state::Removal::Done),
            }
            testing::flush();
            assert!(testing::store().keys().all(|k| !k.starts_with(root().kind(0).entry(&3u32).as_bytes())), "how={how}");
        }
    }

    /// `reserved(2)` makes tag 2 known-and-empty: removal proceeds. It is the developer's
    /// statement that the version that owned the collection cleared it; if it did not (as
    /// here, deliberately), the child keys are orphaned exactly as before round 5. Documented.
    /// Verification round 2, the round-5 false positive fixed by the wire type. Version D writes
    /// two scalars A does not know: a `u8` with a marker value (`0x81..=0x87`, which round 5
    /// mistook for a kind marker) and a `[u8; 3]` whose bytes parse as a record holding a
    /// marker (`09 01 81` in the new format, `02 01 81` in the round-5 one, which round 5's
    /// nested-record search matched). Both are wire-0 scalars, so A's `remove`, `discard`,
    /// `remove_step` and a `LookupMap` `remove` all succeed and leave no key behind.
    #[test]
    fn a3_scalars_with_marker_bytes_never_block_removal() {
        testing::use_mock();
        let mut case = 0u32;
        for small in 0x81u8..=0x87 {
            for shaped in [[0x09, 0x01, small], [0x02, 0x01, small]] {
                for how in 0..4 {
                    case += 1;
                    testing::begin_call();
                    if how < 3 {
                        Map::<u32, RecD>::__bound_at(root()).insert(case, RecD { n: 1, small, shaped });
                    } else {
                        LookupMap::<u32, RecD>::__bound_at(root().field(1)).insert(case, RecD { n: 1, small, shaped });
                    }
                    testing::flush();
                    testing::begin_call();
                    let r = quiet(|| match how {
                        0 => assert_eq!(Map::<u32, RecA>::__bound_at(root()).remove(&case).map(|r| r.n), Some(1)),
                        1 => assert!(Map::<u32, RecA>::__bound_at(root()).discard(&case)),
                        2 => assert_eq!(
                            Map::<u32, RecA>::__bound_at(root()).remove_step(&case, std::num::NonZeroU32::new(3).unwrap()),
                            near_sdk::state::Removal::Done
                        ),
                        _ => assert_eq!(LookupMap::<u32, RecA>::__bound_at(root().field(1)).remove(&case).map(|r| r.n), Some(1)),
                    });
                    assert!(r.is_ok(), "small={small:#x} shaped={shaped:02x?} how={how}: {}", r.err().map(msg).unwrap_or_default());
                    testing::flush();
                }
            }
        }
        // only the two maps' own bookkeeping is left (headers), no record key
        let rec_prefix = |p: Path| p.kind(0).as_bytes().to_vec();
        for k in testing::store().keys() {
            for p in [rec_prefix(root()), rec_prefix(root().field(1))] {
                assert!(!k.starts_with(&p) || k.len() <= p.len() + 1, "left behind: {k:02x?}");
            }
        }
    }

    #[test]
    fn a3_reserved_collection_tag_is_known_and_empty() {
        write_b();
        testing::begin_call();
        let mut a: Map<u32, RecAReserved> = Map::__bound_at(root());
        assert_eq!(a.remove(&1).map(|r| r.n), Some(1));
        testing::flush();
        assert!(testing::store().contains_key(&kid_key()), "reserved(2) trusts that B cleared tag 2; it did not, so kids[5] is left");
    }

    /// Nested records: a nested record's own collection fields leave markers inside its bytes.
    /// (1) The outer record knows the nested field, but as a type that does not declare the
    /// nested collection: refused, tag path `2 > 2`. (2) The outer record does not declare the
    /// nested field at all: the unknown value parses as a record holding a marker, refused,
    /// tag path `2 > 2` (the heuristic for unknown nested records, spec Records).
    #[test]
    fn a3_nested_record_collections_are_checked_at_any_depth() {
        testing::use_mock();
        let mut b: Map<u32, OuterB> = Map::__bound_at(root());
        b.try_insert(1, OuterB { n: 1, ..Default::default() }).unwrap();
        b.at_mut(&1).unwrap().inner.kids.insert(5, 50);
        testing::flush();
        for which in 0..2 {
            testing::begin_call();
            let before = testing::store();
            let e = quiet(|| {
                if which == 0 {
                    drop(Map::<u32, OuterKnowsInnerA>::__bound_at(root()).remove(&1));
                } else {
                    let _ = Map::<u32, OuterA>::__bound_at(root()).discard(&1);
                }
            })
            .expect_err("nested removal accepted");
            assert!(msg(e).contains("carry tag 2 > 2, a `Map` field"), "which={which}");
            testing::begin_call();
            assert_eq!(testing::store(), before);
        }
        testing::begin_call();
        let mut b: Map<u32, OuterB> = Map::__bound_at(root());
        assert!(b.remove(&1).is_some());
        testing::flush();
        assert!(testing::store().keys().all(|k| !k.starts_with(root().kind(0).as_bytes())), "OuterB removes its nested collection");
    }
}

// ---------------------------------------------------------------------------------------
// A5: bounded clearing liveness. API round 5: the budget counts host removes.
// ---------------------------------------------------------------------------------------

mod a5 {
    use near_sdk::near;
    use near_sdk::state::testing::{self, OpKind};
    use near_sdk::state::{Lazy, LookupMap, Map, Path, Removal};
    use std::num::NonZeroU32;

    const BIG: usize = 64 * 1024;

    #[near(storage)]
    #[derive(Default)]
    pub struct Heavy {
        #[key(1)]
        a: Lazy<Vec<u8>>,
        #[key(2)]
        b: Lazy<Vec<u8>>,
        #[key(3)]
        c: Lazy<Vec<u8>>,
        #[key(4)]
        d: Lazy<Vec<u8>>,
        #[key(5)]
        items: Map<u8, u8>,
    }

    /// K = 6 large `Lazy` fields and nothing else.
    #[near(storage)]
    #[derive(Default)]
    pub struct LazySix {
        #[key(1)]
        a: Lazy<Vec<u8>>,
        #[key(2)]
        b: Lazy<Vec<u8>>,
        #[key(3)]
        c: Lazy<Vec<u8>>,
        #[key(4)]
        d: Lazy<Vec<u8>>,
        #[key(5)]
        e: Lazy<Vec<u8>>,
        #[key(6)]
        f: Lazy<Vec<u8>>,
    }
    const K: u32 = 6;

    fn removed(r: &testing::Receipt) -> (usize, usize) {
        let rm: Vec<_> = r.ops.iter().filter(|o| o.kind == OpKind::Remove).collect();
        (rm.len(), rm.iter().map(|o| o.value_len).sum())
    }

    fn fill(s: &mut LazySix) {
        for l in [&mut s.a, &mut s.b, &mut s.c, &mut s.d, &mut s.e, &mut s.f] {
            l.set(vec![7; BIG]);
        }
    }

    /// Calls a record of K one-remove units (the `Lazy` values) needs when finishing costs
    /// `fin` removes: one if everything fits, else the move to the flag key (1) is paid once
    /// and the units pack whole: ceil((K + 1 + fin) / n). For large K that is ceil(K / n).
    fn want_calls(n: u32, fin: u32) -> u32 {
        if K + fin <= n { 1 } else { (K + 1 + fin).div_ceil(n) }
    }

    /// The A5 pin, flipped. A `Lazy` value's remove now counts against the budget like any
    /// other host remove. A record of K = 6 `Lazy` values of 64 KiB, removed with
    /// `remove_step(k, n)` for every n in 3..=10, through a `LookupMap` (finishing is 1 remove,
    /// the record) and a `Map` (3: record, index, key slot): every call issues at most n
    /// removes and so at most n x 64 KiB of value bytes, and the clear takes exactly
    /// `want_calls(n, fin)` calls, ceil(K / n) plus the move and the finish. (Before round 5
    /// the first call removed every `Lazy` value whatever n, and with n = 0 it finished.)
    #[test]
    fn a5_lazy_values_count_against_the_budget() {
        for n in 3..=10u32 {
            for via_map in [false, true] {
                testing::use_mock();
                let root = Path::root(0xF0).field(9);
                if via_map {
                    let mut m: Map<u32, LazySix> = Map::__bound_at(root.clone());
                    m.try_insert(1, LazySix::default()).unwrap();
                    fill(m.get_mut(&1).unwrap());
                } else {
                    let mut m: LookupMap<u32, LazySix> = LookupMap::__bound_at(root.clone());
                    m.try_insert(1, LazySix::default()).unwrap();
                    fill(m.get_mut(&1).unwrap());
                }
                testing::flush();
                let mut per_call = Vec::new();
                loop {
                    testing::begin_call();
                    let nz = NonZeroU32::new(n).unwrap();
                    let more = if via_map {
                        Map::<u32, LazySix>::__bound_at(root.clone()).remove_step(&1, nz)
                    } else {
                        LookupMap::<u32, LazySix>::__bound_at(root.clone()).remove_step(&1, nz)
                    } == Removal::Pending;
                    testing::flush();
                    let rc = testing::receipt();
                    // the stated bookkeeping bound, F = 0 collections, L = 6 Lazy fields
                    assert!(rc.reads() <= (n / 3 + 5) as usize && rc.has_keys() <= 6 && rc.writes() <= 3, "n={n}: {:?}", rc.rwd());
                    per_call.push(removed(&rc));
                    if !more {
                        break;
                    }
                    assert!(per_call.len() < 20);
                }
                let fin = if via_map { 3 } else { 1 };
                assert_eq!(per_call.len() as u32, want_calls(n, fin), "n={n} map={via_map}: {per_call:?}");
                for (rm, bytes) in &per_call {
                    assert!(*rm as u32 <= n, "n={n} map={via_map}: {per_call:?}");
                    assert!(*bytes <= n as usize * BIG + 64, "n={n}: {bytes} value bytes in one call");
                }
                let total: usize = per_call.iter().map(|(r, _)| r).sum();
                let moved = usize::from(per_call.len() > 1);
                assert_eq!(total as u32, K + fin + moved as u32, "n={n} map={via_map}");
                // nothing under the record; a `Map` keeps its emptied header (T4), nothing else
                let rec = root.kind(0).entry(&1u32);
                let header = root.kind(3);
                assert!(
                    testing::store().keys().all(|k| !k.starts_with(root.as_bytes()) || (via_map && k == header.as_bytes())),
                    "n={n}: keys left under {rec:?}"
                );
            }
        }
    }

    /// The original A5 shape (4 x 64 KiB `Lazy` fields and a map of 3 entries) at n = 3: (removes,
    /// value bytes) per call is [(1, 0), (3, 3 x 64 KiB), (1, 64 KiB), ...]: never more than 3
    /// removes or 3 x 64 KiB in one call. Before round 5 at n = 1 it was
    /// [(8, 262166), (7, 6), (11, 26)]: all 256 KiB in the first call.
    #[test]
    fn a5_original_shape_per_call_bytes() {
        testing::use_mock();
        let root = Path::root(0xF0).field(9);
        let mut m: Map<u32, Heavy> = Map::__bound_at(root.clone());
        m.try_insert(1, Heavy::default()).unwrap();
        {
            let h = m.get_mut(&1).unwrap();
            for l in [&mut h.a, &mut h.b, &mut h.c, &mut h.d] {
                l.set(vec![7; BIG]);
            }
            for i in 0..3 {
                h.items.insert(i, i);
            }
        }
        testing::flush();
        let mut per_call = Vec::new();
        loop {
            testing::begin_call();
            let mut m: Map<u32, Heavy> = Map::__bound_at(root.clone());
            let again = m.remove_step(&1, NonZeroU32::new(3).unwrap()) == Removal::Pending;
            testing::flush();
            per_call.push(removed(&testing::receipt()));
            if !again {
                break;
            }
        }
        println!("removes and value bytes per call at n = 3: {per_call:?}");
        assert!(per_call.iter().all(|(rm, bytes)| *rm <= 3 && *bytes <= 3 * BIG + 64), "{per_call:?}");
        // move (1); a, b (2, and c would be a 3rd: it fits, 3 values); d; items one per call;
        // the items header; the record, index and key slot
        let total: usize = per_call.iter().map(|(r, _)| r).sum();
        assert_eq!(total, 1 + 4 + 3 * 3 + 1 + 3);
        assert_eq!(per_call.len(), 7, "{per_call:?}");
    }
}

// ---------------------------------------------------------------------------------------
// C2 for every nestable collection type (the lib fixture nests only a Map)
// ---------------------------------------------------------------------------------------

mod c2_nested {
    use near_sdk::near;
    use near_sdk::state::testing;
    use near_sdk::state::{Lazy, Map, Path, Set, Vector};

    #[near(storage)]
    #[derive(Default)]
    pub struct Deep {
        #[key(1)]
        n: u64,
        #[key(2)]
        v: Vector<u8>,
        #[key(3)]
        s: Set<u8>,
        #[key(4)]
        l: Lazy<Vec<u8>>,
        #[key(5)]
        m: Map<u8, u8>,
    }

    /// Random engine, 1,000 cases: records 0 (neighbour) and 1 each hold a Vector, a Set, a
    /// Lazy and a Map with 0..=5 entries; record 1 is removed by `remove`, `discard` or
    /// `remove_step(n)` (n = 3..=5 host removes, API round 5; repeated until done). No stored key starts with record 1's
    /// path afterwards, and record 0's collections are intact.
    #[test]
    fn c2_every_nested_collection_type_is_cleared_with_its_record() {
        let g = (bolero::produce::<[u8; 4]>(), 0u8..3, 3u32..=5);
        bolero::check!().with_generator(g).with_iterations(1_000).for_each(|(sizes, how, n)| {
            testing::use_mock();
            let root = Path::root(0xF0).field(9);
            let mut m: Map<u32, Deep> = Map::__bound_at(root.clone());
            for r in 0..2u32 {
                m.try_insert(r, Deep { n: 1, ..Default::default() }).unwrap();
                let d = m.get_mut(&r).unwrap();
                for i in 0..sizes[0] % 6 {
                    d.v.push(i);
                }
                for i in 0..sizes[1] % 6 {
                    d.s.insert(i);
                }
                if sizes[2] % 2 == 1 {
                    d.l.set(vec![9; 40]);
                }
                for i in 0..sizes[3] % 6 {
                    d.m.insert(i, i);
                }
            }
            testing::flush();
            let snapshot0: Vec<_> = testing::store().into_iter().filter(|(k, _)| k.starts_with(root.kind(0).entry(&0u32).as_bytes())).collect();
            for round in 0.. {
                testing::begin_call();
                let mut m: Map<u32, Deep> = Map::__bound_at(root.clone());
                let again = match how {
                    0 => m.remove(&1).is_some() && false,
                    1 => m.discard(&1) && false,
                    _ => m.remove_step(&1, std::num::NonZeroU32::new(*n).unwrap()) == near_sdk::state::Removal::Pending,
                };
                testing::flush();
                if *how == 2 {
                    // API round 5: at most n removes; the stated bookkeeping bound with F = 3
                    // (Vector, Set, Map) and L = 1
                    let rc = testing::receipt();
                    assert!(rc.removes() <= *n as usize, "n={n}: {} removes", rc.removes());
                    assert!(
                        rc.reads() <= (*n / 3 + 3 + 5) as usize && rc.has_keys() <= 1 && rc.writes() <= 3,
                        "n={n}: reads, writes, removes {:?}, has_keys {}",
                        rc.rwd(),
                        rc.has_keys()
                    );
                }
                if !again {
                    break;
                }
                // at most 43 removes at 1..=n per call
                assert!(round < 45, "remove_step did not finish");
            }
            let gone = root.kind(0).entry(&1u32);
            let left: Vec<_> = testing::store().into_keys().filter(|k| k.starts_with(gone.as_bytes())).collect();
            assert!(left.is_empty(), "how={how} left {left:02x?} under the removed record (sizes {sizes:?})");
            let now0: Vec<_> = testing::store().into_iter().filter(|(k, _)| k.starts_with(root.kind(0).entry(&0u32).as_bytes())).collect();
            assert_eq!(now0, snapshot0, "the neighbour record changed");
        });
    }
}
