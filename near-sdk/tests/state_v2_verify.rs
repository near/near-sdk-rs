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

    /// The stated exception, pinned as a hazard: a v1 map NOT wrapped in `Migrating` (a v1
    /// field kept beside the v2 root) is not checked. With an empty prefix and a `[u8; 1]`
    /// key, a caller-chosen key `[0xF0]` overwrites the v2 root record.
    #[test]
    fn hazard_an_unwrapped_empty_prefix_v1_map_overwrites_the_v2_root() {
        fresh();
        let c = C { m: Migrating::new(V1LookupMap::new(b"s".to_vec())), n: 7 };
        c.state_write();
        drop(c);
        let mut bare: V1LookupMap<[u8; 1], u8> = V1LookupMap::new(vec![]);
        bare.set([0xF0], Some(0)); // `set` writes without reading the old value
        bare.flush();
        assert_eq!(env::storage_read(&[0xF0]).unwrap(), vec![0], "HAZARD: the v2 root was overwritten by a v1 entry");
        testing::begin_call();
        assert!(std::panic::catch_unwind(|| C::state_read().map(|c| c.n)).is_err(), "the contract no longer loads");
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
        }

        fn build(pa: &[u8], pb: &[u8], pc: &[u8]) -> T {
            T { a: Migrating::new(TreeMap::new(pa.to_vec())), b: Migrating::new(UnorderedMap::new(pb.to_vec())), c: Migrating::new(IterableMap::with_hasher(pc.to_vec())) }
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
        }
    }
}
