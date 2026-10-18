//! Storage v2 migration round: `Migrating` over iterable v1 maps (`migrate_batch`, the wrapper
//! coming off), and the root byte against v1 keys a caller can shape.
#![cfg(feature = "unstable")]

use near_sdk::collections::TreeMap;
use near_sdk::state::testing::{self, OpKind};
use near_sdk::state::{ContractState, Map, Migrating, StateRead, StateWrite};
use near_sdk::store::IterableMap;
use near_sdk::test_utils::VMContextBuilder;
use near_sdk::{AccountId, BorshStorageKey, env, near, testing_env};

const ROOT: u8 = 0xF0;

fn ctx() {
    testing_env!(VMContextBuilder::new().storage_usage(env::storage_usage()).build());
}

fn call<T: ContractState + StateRead + StateWrite, R>(f: impl FnOnce(&mut T) -> R) -> (R, testing::Receipt) {
    ctx();
    testing::begin_call();
    testing::env_log_take();
    let mut c = T::state_read().unwrap();
    let r = f(&mut c);
    c.state_write();
    drop(c);
    (r, testing::env_log_take())
}

fn init<T: ContractState + StateWrite>(f: impl FnOnce() -> T) {
    ctx();
    testing::begin_call();
    let c = f();
    c.state_write();
    drop(c);
}

fn tok(i: u32) -> String {
    format!("token-{i:04}")
}
fn owner(i: u32) -> AccountId {
    format!("owner{}.near", i % 7).parse().unwrap()
}

// ---- v1: the NFT standard's `owner_by_id` shape (a `collections::TreeMap`) and a
// `store::IterableMap`, at a v1 Borsh root ----

#[derive(BorshStorageKey)]
#[near]
enum V1Key {
    OwnerById,
    Approvals,
}

#[near(contract_state)]
pub struct NftV1 {
    owner_by_id: TreeMap<String, AccountId>,
    approvals: IterableMap<String, u64>,
}
impl Default for NftV1 {
    fn default() -> Self {
        Self { owner_by_id: TreeMap::new(V1Key::OwnerById), approvals: IterableMap::new(V1Key::Approvals) }
    }
}

const N: u32 = 120;

fn populate() {
    ctx();
    testing::use_env_host();
    testing::env_log_start();
    let mut c = NftV1::default();
    for i in 0..N {
        c.owner_by_id.insert(&tok(i), &owner(i));
        c.approvals.insert(tok(i), i as u64);
    }
    env::state_write(&c);
    drop(c);
    assert!(testing::env_store().keys().all(|k| k[0] != ROOT));
}

mod during {
    use super::*;
    pub type Owners = Migrating<TreeMap<String, AccountId>, Map<String, AccountId>>;
    pub type Approvals = Migrating<IterableMap<String, u64>, Map<String, u64>>;

    #[near(contract_state(layout = v2))]
    pub struct NftV2 {
        #[key(1, required)]
        pub owner_by_id: Owners,
        #[key(2, required)]
        pub approvals: Approvals,
    }
    impl NftV2 {
        pub fn migrate() -> Self {
            let old: NftV1 = env::state_read().unwrap();
            env::storage_remove(b"STATE");
            Self { owner_by_id: Migrating::new(old.owner_by_id), approvals: Migrating::new(old.approvals) }
        }
    }
}
use during::NftV2;

mod after {
    use super::*;
    /// The wrappers off: the same tags, now bare v2 maps.
    #[near(contract_state(layout = v2))]
    pub struct NftV3 {
        #[key(1)]
        pub owner_by_id: Map<String, AccountId>,
        #[key(2)]
        pub approvals: Map<String, u64>,
    }
}
use after::NftV3;

#[test]
fn migrate_batch_drains_iterable_v1_maps_and_the_wrapper_comes_off() {
    populate();
    init(NftV2::migrate);

    // Lazy path first: a read of an unmigrated token falls back.
    let (o, r) = call::<NftV2, _>(|c| c.owner_by_id.get(&tok(3)));
    assert_eq!(o, Some(owner(3)));
    println!("TreeMap fallback read: {} reads (root + new miss + TreeMap get)", r.reads());

    // Batches of 25 across calls, until the old side is empty.
    let mut calls = 0;
    loop {
        let (done, r) = call::<NftV2, _>(|c| {
            let a = c.owner_by_id.migrate_batch(25);
            let b = c.approvals.migrate_batch(25);
            (a + b, c.owner_by_id.old_is_empty() && c.approvals.old_is_empty())
        });
        calls += 1;
        if calls == 1 {
            println!(
                "first batch (25 + 25): rwd {:?}, has_key {}, max value {} B",
                r.rwd(),
                r.has_keys(),
                r.max_value_bytes()
            );
        }
        if done.1 {
            break;
        }
        assert!(calls < 20);
    }
    println!("drained {N} + {N} entries in {calls} calls");
    let store = testing::env_store();
    // Only v2 keys remain (the TreeMap and IterableMap leave nothing, STATE is gone).
    let v1_left: Vec<_> = store.keys().filter(|k| k[0] != ROOT).collect();
    assert!(v1_left.is_empty(), "{} v1 keys left: {:02x?}", v1_left.len(), v1_left.iter().take(3).collect::<Vec<_>>());

    // The wrapper comes off: redeploy with bare `Map`s at the same tags. No migrate needed.
    let (ok, r) = call::<NftV3, _>(|c| {
        (0..N).all(|i| c.owner_by_id.get(&tok(i)) == Some(&owner(i)) && c.approvals.get(&tok(i)) == Some(&(i as u64)))
    });
    assert!(ok);
    assert_eq!(r.writes(), 0, "nothing to rewrite");
    let (len, _) = call::<NftV3, _>(|c| (c.owner_by_id.len(), c.approvals.len()));
    assert_eq!(len, (N, N));

    // The root record still carries the v1 prefixes under tags 1 and 2: the encoder keeps a
    // stored tag the code knows as non-inline. It is never rewritten while no root scalar
    // changes, and kept (merged) when one does.
    let root = &testing::env_store()[&vec![ROOT]];
    let tags: Vec<u32> = near_sdk::state::record::parse(root).unwrap().iter().map(|(t, _)| *t).collect();
    println!("root after the wrapper came off: {} bytes, tags {tags:?}", root.len());
    assert_eq!(tags, vec![1, 2], "stale v1 prefix bytes stay in the root");
}

/// Writes through the wrapper keep the one-side invariant, so `migrate_batch` never
/// overwrites a newer value with an older one.
#[test]
fn write_then_batch_keeps_the_newer_value() {
    populate();
    init(NftV2::migrate);
    let new_owner: AccountId = "buyer.near".parse().unwrap();
    call::<NftV2, _>(|c| c.owner_by_id.insert(tok(0), new_owner.clone()));
    let store = testing::env_store();
    assert!(store.keys().filter(|k| k[0] != ROOT).count() > 0);
    loop {
        let (done, _) = call::<NftV2, _>(|c| {
            c.owner_by_id.migrate_batch(50);
            c.owner_by_id.old_is_empty()
        });
        if done {
            break;
        }
    }
    let (o, _) = call::<NftV2, _>(|c| c.owner_by_id.get(&tok(0)));
    assert_eq!(o, Some(new_owner));
}

// ---------------------------------------------------------------------------------------
// The root byte vs v1 keys a caller can shape
// ---------------------------------------------------------------------------------------

/// The spec's guards for the root byte are a default outside small enum prefixes and a scan
/// of existing state. Neither covers a v1 map whose prefix is empty (a unit-struct
/// `BorshStorageKey`, as in `examples/status-message`) keyed by a variable-length value the
/// caller picks: borsh(Vec<u8>) starts with its u32 length, so a key 0x1F0 = 496 bytes long
/// starts with F0 01 00 00, after any scan. Here a caller turns a v1 write into a write of
/// their own v2 balance record.
mod root_byte {
    use super::*;
    use near_sdk::store::LookupMap as V1LookupMap;

    #[derive(BorshStorageKey)]
    #[near]
    struct Notes;

    #[near(contract_state(layout = v2))]
    pub struct App {
        /// v2, balances keyed by a caller-chosen handle.
        #[key(1)]
        pub balances: near_sdk::state::LookupMap<String, u128>,
        /// v1, empty prefix, caller-chosen byte keys and values (a guestbook, a registry).
        #[key(2, legacy)]
        pub notes: V1LookupMap<Vec<u8>, Vec<u8>>,
    }

    #[test]
    fn empty_prefix_v1_map_with_caller_keys_can_write_a_v2_key() {
        ctx();
        testing::use_env_host();
        testing::env_log_start();
        init(|| App { balances: Default::default(), notes: V1LookupMap::new(Notes) });

        // v2 key: F0, tag 1, ENTRIES 00, h16(k), borsh(k). The v1 key is borsh(bytes) =
        // u32 LE length ++ bytes. They are equal when the first four v2 bytes read as the
        // length: F0 01 00 h16[0] with h16[0] = 0 gives 496, so the v2 key must be 500 bytes:
        // 3 + 16 + 4 + len(k) = 500, len(k) = 477 (under the 512-byte String bound). Grinding
        // h16[0] = 0 takes ~256 hashes.
        let (handle, v2_key) = (0u32..)
            .map(|n| {
                let k = format!("{n:0>477}");
                let key = near_sdk::state::Path::root(ROOT).field(1).kind(0).entry(&k);
                (k, key.as_bytes().to_vec())
            })
            .find(|(_, key)| key[3] == 0)
            .unwrap();
        assert_eq!(v2_key.len(), 500);
        assert_eq!(&v2_key[..4], &[ROOT, 0x01, 0x00, 0x00]);
        let v1_key = v2_key[4..].to_vec();
        assert_eq!(near_sdk::borsh::to_vec(&v1_key).unwrap(), v2_key);

        // borsh(Vec<u8>) of 12 bytes is 16 bytes: a u128 to the v2 decoder.
        let (_, r) = call::<App, _>(|c| {
            c.notes.insert(v1_key.clone(), vec![0xFF; 12]);
        });
        assert!(r.ops.iter().any(|o| o.kind == OpKind::Write && o.key == v2_key));
        let (bal, _) = call::<App, _>(|c| c.balances.get(&handle).copied());
        println!("forged v2 balance for the attacker's handle: {bal:?}");
        assert!(bal.unwrap() > 1u128 << 100);
    }
}

#[test]
fn migrating_lookup_map_has_no_migrate_batch() {
    // Compile probe recorded in PROTOTYPE_NOTES.md ("Migration round"):
    //   let m: Migrating<near_sdk::collections::LookupMap<String, u64>, Map<String, u64>> = ..;
    //   m.migrate_batch(10);
    // fails with E0599 and the `LegacyIterable` diagnostic. Nothing to run here.
    let _ = OpKind::Read;
}

/// `Migrating<store::LookupMap, _>`: the only legacy delete that is deferred. `set(k, None)`
/// lands at the v1 map's `Drop`, after the v2 flush (root included).
mod store_lookup {
    use super::*;
    use near_sdk::store::LookupMap as V1LookupMap;

    #[derive(BorshStorageKey)]
    #[near]
    enum K {
        Scores,
    }

    #[near(contract_state)]
    pub struct V1 {
        scores: V1LookupMap<String, u64>,
    }

    mod v2m {
        use super::*;
        #[near(contract_state(layout = v2))]
        pub struct V2 {
            #[key(1, required)]
            pub scores: Migrating<V1LookupMap<String, u64>, Map<String, u64>>,
            #[key(2)]
            pub writes: u64,
        }
    }
    use v2m::V2;

    fn v1_key(k: &str) -> Vec<u8> {
        [vec![0u8], near_sdk::borsh::to_vec(&k.to_string()).unwrap()].concat()
    }

    #[test]
    fn store_lookup_map_delete_lands_at_drop_after_the_v2_flush() {
        ctx();
        testing::use_env_host();
        testing::env_log_start();
        let mut c = V1 { scores: V1LookupMap::new(K::Scores) };
        c.scores.insert("alice".into(), 7);
        env::state_write(&c);
        drop(c);
        init(|| {
            let old: V1 = env::state_read().unwrap();
            env::storage_remove(b"STATE");
            V2 { scores: Migrating::new(old.scores), writes: 0 }
        });

        // One write through the wrapper, phases split as the wrapper runs them.
        ctx();
        testing::begin_call();
        testing::env_log_take();
        let mut c = V2::state_read().unwrap();
        assert_eq!(c.scores.get(&"alice".into()), Some(7), "read falls back to v1");
        c.scores.insert("alice".into(), 8);
        c.scores.insert("bob".into(), 1); // never on either side: blind old-side delete
        c.writes += 1;
        let body = testing::env_log_take().ops;
        c.state_write();
        let flush = testing::env_log_take().ops;
        drop(c);
        let dropped = testing::env_log_take().ops;
        let show = |ops: &[testing::Op]| ops.iter().map(|o| format!("{:?} {:02x?}", o.kind, &o.key[..o.key.len().min(4)])).collect::<Vec<_>>();
        println!("body {:?}\nflush {:?}\ndrop {:?}", show(&body), show(&flush), show(&dropped));

        assert!(body.iter().all(|o| o.kind == OpKind::Read), "nothing written in the body");
        assert!(flush.iter().all(|o| o.kind == OpKind::Write && o.key[0] == ROOT));
        assert_eq!(flush.last().unwrap().key, vec![ROOT], "root last");
        // At drop: alice's v1 entry removed, and a storage_remove for bob, who never existed.
        let removes: Vec<_> = dropped.iter().filter(|o| o.kind == OpKind::Remove).map(|o| o.key.clone()).collect();
        assert_eq!(removes, vec![v1_key("alice"), v1_key("bob")]);
        assert!(!env::storage_has_key(&v1_key("alice")));

        let (v, _) = call::<V2, _>(|c| (c.scores.get(&"alice".into()), c.scores.get(&"bob".into())));
        assert_eq!(v, (Some(8), Some(1)));
    }
}

// ---------------------------------------------------------------------------------------
// Audit round (Oct 17, Codex claims). A4: `new_side()` / `old_side()` handed out the raw maps,
// so a caller could break the one-side invariant that `migrate_batch` and `remove` relied on.
// API round 4: the side accessors are read-only; the mutable ones are named
// `*_side_mut_unchecked` (they bypass the rule), and `migrate_batch` / `remove` no longer
// trust the rule. These tests break it on purpose through the unchecked accessors and assert
// the fixed behaviour.
// ---------------------------------------------------------------------------------------

fn drain_owners() {
    for _ in 0..20 {
        let (done, _) = call::<NftV2, _>(|c| {
            c.owner_by_id.migrate_batch(50);
            c.owner_by_id.old_is_empty()
        });
        if done {
            return;
        }
    }
    panic!("old side did not drain");
}

/// A4, leg 1: a newer value written straight to the new side while the old side still holds
/// the key. `migrate_batch` used to overwrite it with the stale v1 value; it now drops the old
/// entry because the key is already on the new side.
#[test]
fn audit_a4_new_side_write_is_overwritten_by_migrate_batch() {
    populate();
    init(NftV2::migrate);
    let buyer: AccountId = "buyer.near".parse().unwrap();
    call::<NftV2, _>(|c| {
        let _ = c.owner_by_id.new_side_mut_unchecked().insert(tok(0), buyer.clone());
    });
    let (o, _) = call::<NftV2, _>(|c| c.owner_by_id.get(&tok(0)));
    assert_eq!(o, Some(buyer.clone()), "before the batch, the new side wins the read");
    drain_owners();
    let (o, _) = call::<NftV2, _>(|c| c.owner_by_id.get(&tok(0)));
    assert_eq!(o, Some(buyer), "the batch skipped the key already on the new side");
    let (n, _) = call::<NftV2, _>(|c| c.owner_by_id.new_side().len());
    assert_eq!(n, N, "every token on the new side once");
}

/// A4, leg 2: same setup, then `remove(k)` on the wrapper. It used to remove only from the new
/// side, so the old value resurrected through the fallback read (and the batch made it
/// permanent); it now deletes from both sides.
#[test]
fn audit_a4_remove_after_new_side_write_resurrects_the_old_value() {
    populate();
    init(NftV2::migrate);
    let buyer: AccountId = "buyer.near".parse().unwrap();
    call::<NftV2, _>(|c| {
        let _ = c.owner_by_id.new_side_mut_unchecked().insert(tok(1), buyer.clone());
    });
    let (removed, _) = call::<NftV2, _>(|c| c.owner_by_id.remove(&tok(1)));
    assert_eq!(removed, Some(buyer), "the new side's value comes back");
    let (o, _) = call::<NftV2, _>(|c| c.owner_by_id.get(&tok(1)));
    assert_eq!(o, None, "no resurrection: the old side lost tok(1) too");
    drain_owners();
    let (o, _) = call::<NftV2, _>(|c| c.owner_by_id.get(&tok(1)));
    assert_eq!(o, None, "and the batch has nothing to bring back");
    let (n, _) = call::<NftV2, _>(|c| c.owner_by_id.new_side().len());
    assert_eq!(n, N - 1);
}

/// A4, the mirror through the old side: a stale value put back on the old side after the key
/// moved to the new side. The next batch used to overwrite the new value with it; it now drops
/// the stale entry.
#[test]
fn audit_a4_old_side_write_overwrites_the_new_value_at_the_next_batch() {
    populate();
    init(NftV2::migrate);
    let buyer: AccountId = "buyer.near".parse().unwrap();
    let stale: AccountId = "stale.near".parse().unwrap();
    call::<NftV2, _>(|c| c.owner_by_id.insert(tok(2), buyer.clone()));
    call::<NftV2, _>(|c| {
        c.owner_by_id.old_side_mut_unchecked().insert(&tok(2), &stale);
    });
    let (o, _) = call::<NftV2, _>(|c| c.owner_by_id.get(&tok(2)));
    assert_eq!(o, Some(buyer.clone()), "new side still wins the read");
    drain_owners();
    let (o, _) = call::<NftV2, _>(|c| c.owner_by_id.get(&tok(2)));
    assert_eq!(o, Some(buyer), "the stale old entry was dropped, not moved");
    let (gone, _) = call::<NftV2, _>(|c| c.owner_by_id.old_side().get(&tok(2)).is_none());
    assert!(gone, "and it is gone from the old side");
}

/// API round 4 cost of the A4 `remove` fix, pinned: removing a key that already moved to the
/// new side now also removes it on the old side. The round-3 path (read, then remove on the new
/// side only) is replayed by hand on a second moved key for the comparison.
#[test]
fn remove_of_a_moved_key_also_removes_on_the_old_side() {
    populate();
    init(NftV2::migrate);
    let buyer: AccountId = "buyer.near".parse().unwrap();
    call::<NftV2, _>(|c| {
        c.owner_by_id.insert(tok(3), buyer.clone());
        c.owner_by_id.insert(tok(4), buyer.clone());
    });
    let (v, before) = call::<NftV2, _>(|c| {
        let v = c.owner_by_id.get(&tok(4));
        assert!(c.owner_by_id.new_side_mut_unchecked().discard(&tok(4)));
        v
    });
    assert_eq!(v, Some(buyer.clone()));
    let (removed, after) = call::<NftV2, _>(|c| c.owner_by_id.remove(&tok(3)));
    assert_eq!(removed, Some(buyer));
    assert_eq!(before.rwd(), (4, 1, 3), "the round-3 path: read, then remove on the new side only");
    assert_eq!(after.rwd(), (5, 1, 3), "+1 read: the v1 TreeMap's contains_key, which misses");
    assert_eq!((before.has_keys(), after.has_keys()), (0, 0));
}
