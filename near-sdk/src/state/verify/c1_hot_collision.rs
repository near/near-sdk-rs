//! Bug class C1, HOT-style collision: two collections' keys meeting in one raw key space.
//! In v1 a map's key is `prefix ++ borsh(key)`, so one map's (prefix, key) can spell another's
//! (the hot-bridge withdraw collision). In v2 every key is `path ++ kind ++ h(enc) ++ enc`
//! with the path prefix-free (G1). Here the adversary chooses keys by SPLICING other
//! collections' full encoded storage keys (and their tails) into variable-length `Vec<u8>` and
//! `String` keys, across sibling maps at adjacent tags, a `LookupMap`, and a map nested in a
//! record. Every (collection, key) pair keeps its own storage key and reads its own value.
//! The general claim (any two distinct paths, symbolic hashes) is G1's Kani harness.

use super::fixture::Rec;
use crate::state::key::{DEFAULT_ROOT, Path};
use crate::state::testing;
use crate::state::{LookupMap, Map};

#[allow(clippy::type_complexity)]
fn maps()
-> (Map<Vec<u8>, Vec<u8>>, Map<Vec<u8>, Vec<u8>>, LookupMap<String, Vec<u8>>, Map<Vec<u8>, Rec>) {
    let r = Path::root(DEFAULT_ROOT);
    (
        Map::__bound_at(r.field(10)),
        Map::__bound_at(r.field(11)),
        LookupMap::__bound_at(r.field(138)),
        Map::__bound_at(r.field(12)),
    )
}

/// Storage key of key `k` in collection `c` (0..=3; 3 is record `0`'s `items` map, `u8` keys).
fn skey(c: u8, k: &[u8]) -> Vec<u8> {
    let r = Path::root(DEFAULT_ROOT);
    match c {
        0 => r.field(10).kind(0).entry(&k.to_vec()),
        1 => r.field(11).kind(0).entry(&k.to_vec()),
        2 => r.field(138).kind(0).entry(&String::from_utf8_lossy(k).into_owned()),
        _ => r
            .field(12)
            .kind(0)
            .entry(&vec![0u8])
            .field(2)
            .kind(0)
            .entry(&k.first().copied().unwrap_or(0)),
    }
    .as_bytes()
    .to_vec()
}

#[derive(Clone, Debug, bolero::generator::TypeGenerator)]
pub struct Pick {
    #[generator(0..4)]
    pub coll: u8,
    /// Seed key bytes.
    #[generator(bolero::produce::<Vec<u8>>().with().len(0usize..=6))]
    pub raw: Vec<u8>,
    /// Splice: replace the key with the tail (from byte `cut`) of an earlier pick's storage key.
    pub splice: Option<(u8, u8)>,
}

/// Random engine, 5,000 cases of up to 12 picks: each pick's key is either random bytes or a
/// tail of an earlier pick's full storage key (so keys embed root bytes, tags, kind bytes and
/// placement hashes of other collections). All storage keys of distinct (collection, key)
/// pairs differ; after a flush and a new call each pair reads back exactly its own value; and
/// the number of entry keys stored equals the number of distinct pairs.
#[test]
fn c1_spliced_keys_never_collide_across_collections() {
    let g = bolero::produce::<Vec<Pick>>().with().len(1usize..=12);
    bolero::check!().with_generator(g).with_iterations(5_000).for_each(|picks| {
        testing::use_mock();
        let mut chosen: Vec<(u8, Vec<u8>)> = Vec::new();
        for p in picks {
            let key = match p.splice {
                Some((j, cut)) if !chosen.is_empty() => {
                    let (c, k) = &chosen[j as usize % chosen.len()];
                    let full = skey(*c, k);
                    full[(cut as usize) % full.len()..].to_vec()
                }
                _ => p.raw.clone(),
            };
            let key = if p.coll == 2 {
                String::from_utf8_lossy(&key).into_owned().into_bytes()
            } else {
                key
            };
            let key =
                if p.coll == 3 { key.first().map(|b| vec![*b]).unwrap_or(vec![0]) } else { key };
            if !chosen.iter().any(|(c, k)| *c == p.coll && *k == key) {
                chosen.push((p.coll, key));
            }
        }
        // distinct pairs, distinct storage keys
        for (i, a) in chosen.iter().enumerate() {
            for b in &chosen[i + 1..] {
                assert_ne!(skey(a.0, &a.1), skey(b.0, &b.1), "{a:?} and {b:?} share a storage key");
            }
        }
        let val = |i: usize| vec![i as u8, 0xAA];
        let (mut m0, mut m1, mut l2, mut r3) = maps();
        r3.try_insert(vec![0], Rec::default()).unwrap();
        for (i, (c, k)) in chosen.iter().enumerate() {
            match c {
                0 => drop(m0.insert(k.clone(), val(i))),
                1 => drop(m1.insert(k.clone(), val(i))),
                2 => l2.insert(String::from_utf8(k.clone()).unwrap(), val(i)),
                _ => drop(r3.at_mut(&vec![0]).unwrap().items.insert(k[0], val(i))),
            }
        }
        testing::flush();
        testing::begin_call();
        let (m0, m1, l2, r3) = maps();
        for (i, (c, k)) in chosen.iter().enumerate() {
            let got = match c {
                0 => m0.get(k).cloned(),
                1 => m1.get(k).cloned(),
                2 => l2.get(&String::from_utf8(k.clone()).unwrap()).cloned(),
                _ => r3.at(&vec![0]).items.get(&k[0]).cloned(),
            };
            assert_eq!(got, Some(val(i)), "({c}, {k:02x?}) read another collection's value");
        }
        let store = testing::store();
        for (c, k) in &chosen {
            assert!(store.contains_key(&skey(*c, k)));
        }
    });
}

/// The bug class is real in v1 (baseline): `collections::LookupMap` with prefix `"a"` and key
/// `"x"` (`61 ++ 01 00 00 00 78`) and a second map with prefix `"a\x01"` and the 4-byte key
/// `00 00 00 78` write the SAME storage key, so the second map reads the first map's value.
#[test]
fn c1_baseline_v1_raw_prefix_maps_collide() {
    use crate::collections::LookupMap as V1;
    testing::use_env_host();
    crate::testing_env!(crate::test_utils::VMContextBuilder::new().build());
    let mut a: V1<String, u8> = V1::new(b"a".to_vec());
    let b: V1<[u8; 4], u8> = V1::new(b"a\x01".to_vec());
    a.insert(&"x".to_string(), &7);
    assert_eq!(b.get(&[0, 0, 0, 0x78]), Some(7), "v1 baseline: map b reads map a's entry");
}
