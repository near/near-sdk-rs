//! Storage v2 migration round, on the mocked blockchain: the unmodified v1
//! `examples/fungible-token` populates state, then the v2 and v3 contracts in this crate run
//! over the same storage. Every `env::storage_*` call (v1 collections and the v2 cache alike)
//! goes into one ordered log (`state::testing::env_log_*`).

#[path = "../../ft/src/lib.rs"]
#[allow(dead_code, unused_imports)]
mod v1;

use fungible_token_v2::{v2, v3};
use near_contract_standards::fungible_token::FungibleTokenCore;
use near_contract_standards::fungible_token::metadata::{FT_METADATA_SPEC, FungibleTokenMetadata};
use near_contract_standards::storage_management::StorageManagement;
use near_sdk::json_types::U128;
use near_sdk::state::testing::{self, Op, OpKind, Receipt};
use near_sdk::state::{ContractState, StateRead, StateWrite};
use near_sdk::test_utils::VMContextBuilder;
use near_sdk::{AccountId, NearToken, env, testing_env};
use std::collections::BTreeMap;

const CONTRACT: &str = "ft.near";
const OWNER: &str = "owner.near";
const USERS: usize = 300;
const SUPPLY: u128 = 1_000_000_000_000;
const ROOT: u8 = 0xF0;
const ICON_BYTES: usize = 100 * 1024;

fn user(i: usize) -> AccountId {
    format!("user{i:03}.near").parse().unwrap()
}
fn acc(s: &str) -> AccountId {
    s.parse().unwrap()
}

/// The v1 key of an account balance: prefix `[0]` (`StorageKey::FungibleToken`) ++ borsh(id).
fn v1_balance_key(a: &AccountId) -> Vec<u8> {
    let mut k = vec![0u8];
    k.extend(near_sdk::borsh::to_vec(a).unwrap());
    k
}
const V1_METADATA_KEY: &[u8] = &[1u8];

fn ctx(who: &str, deposit: NearToken) {
    let ctx = VMContextBuilder::new()
        .current_account_id(acc(CONTRACT))
        .predecessor_account_id(acc(who))
        .signer_account_id(acc(who))
        .attached_deposit(deposit)
        .storage_usage(env::storage_usage())
        .account_balance(NearToken::from_near(1_000))
        .build();
    testing_env!(ctx);
}

/// What the v1 `#[near]` wrapper does around a mutating method.
fn v1_call<R>(who: &str, deposit: NearToken, f: impl FnOnce(&mut v1::Contract) -> R) -> (R, Receipt) {
    ctx(who, deposit);
    testing::env_log_take();
    let mut c: v1::Contract = env::state_read().expect("v1 initialized");
    let r = f(&mut c);
    env::state_write(&c);
    drop(c);
    (r, testing::env_log_take())
}

/// One call's host log, split where the wrapper's phases end.
#[derive(Debug)]
struct CallLog {
    /// `state_read` and the method body.
    body: Vec<Op>,
    /// `state_write` (the v2 flush, root last).
    flush: Vec<Op>,
    /// Dropping the root after the write (v1 `store` write-on-drop).
    drop: Vec<Op>,
}
impl CallLog {
    fn all(&self) -> Vec<&Op> {
        self.body.iter().chain(&self.flush).chain(&self.drop).collect()
    }
    fn count(&self, k: OpKind) -> usize {
        self.all().iter().filter(|o| o.kind == k).count()
    }
    fn rwd(&self) -> (usize, usize, usize) {
        (self.count(OpKind::Read), self.count(OpKind::Write), self.count(OpKind::Remove))
    }
}

/// What the `#[near]` wrapper does around a mutating method of a v2 root.
fn v2_call<T, R>(who: &str, deposit: NearToken, f: impl FnOnce(&mut T) -> R) -> (R, CallLog)
where
    T: ContractState + StateRead + StateWrite,
{
    ctx(who, deposit);
    testing::begin_call();
    testing::env_log_take();
    let mut c = T::state_read().expect("v2 initialized");
    let r = f(&mut c);
    let body = testing::env_log_take().ops;
    c.state_write();
    let flush = testing::env_log_take().ops;
    drop(c);
    let drop = testing::env_log_take().ops;
    (r, CallLog { body, flush, drop })
}

fn v2_view<T, R>(f: impl FnOnce(&T) -> R) -> (R, Receipt)
where
    T: ContractState + StateRead + StateWrite,
{
    testing::begin_call();
    testing::env_log_take();
    let c = T::state_read().expect("v2 initialized");
    let r = f(&c);
    testing::assert_view_clean();
    drop(c);
    (r, testing::env_log_take())
}

/// What the wrapper does around an `#[init]` method of a v2 root.
fn v2_init<T: ContractState + StateWrite>(ignore_state: bool, f: impl FnOnce() -> T) -> CallLog {
    ctx(CONTRACT, NearToken::from_yoctonear(0));
    testing::begin_call();
    testing::env_log_take();
    if !ignore_state && T::state_exists() {
        env::panic_str("The contract has already been initialized");
    }
    let c = f();
    let body = testing::env_log_take().ops;
    c.state_write();
    let flush = testing::env_log_take().ops;
    drop(c);
    let drop = testing::env_log_take().ops;
    CallLog { body, flush, drop }
}

/// Balances the v1 population leaves behind.
fn expected(i: usize) -> u128 {
    1_000 + i as u128
}

/// Runs the unmodified v1 contract: init with a 100 KiB icon in the metadata, 300 accounts
/// registered through NEP-145 and funded by `ft_transfer` from the owner. Returns the store.
fn populate_v1() -> BTreeMap<Vec<u8>, Vec<u8>> {
    ctx(OWNER, NearToken::from_yoctonear(0));
    testing::use_env_host();
    testing::env_log_start();
    let icon = format!("data:image/svg+xml,{}", "x".repeat(ICON_BYTES));
    let meta = FungibleTokenMetadata {
        spec: FT_METADATA_SPEC.to_string(),
        name: "Example".into(),
        symbol: "EX".into(),
        icon: Some(icon),
        reference: None,
        reference_hash: None,
        decimals: 24,
    };
    let c = v1::Contract::new(acc(OWNER), U128(SUPPLY), meta);
    env::state_write(&c);
    drop(c);

    let bond = v1_call(OWNER, NearToken::from_yoctonear(0), |c| c.storage_balance_bounds().min).0;
    for i in 0..USERS {
        v1_call(user(i).as_str(), bond, |c| c.storage_deposit(None, None));
        v1_call(OWNER, NearToken::from_yoctonear(1), |c| {
            c.ft_transfer(user(i), U128(expected(i)), None)
        });
    }
    testing::env_store()
}

fn transferred() -> u128 {
    (0..USERS).map(expected).sum()
}

fn fmt_ops(ops: &[Op]) -> String {
    ops.iter()
        .map(|o| {
            let k: String = o.key.iter().take(6).map(|b| format!("{b:02x}")).collect();
            format!("  {:?} {k}{} ({}B)", o.kind, if o.key.len() > 6 { ".." } else { "" }, o.value_len)
        })
        .collect::<Vec<_>>()
        .join("\n")
}

// ---------------------------------------------------------------------------------------
// Claim 1 + 2: move the root
// ---------------------------------------------------------------------------------------

#[test]
fn v1_population_never_writes_under_the_root_byte() {
    let store = populate_v1();
    // 300 users + owner, the metadata blob, STATE.
    assert_eq!(store.len(), USERS + 1 + 2, "{:?}", store.keys().map(|k| k.first()).collect::<Vec<_>>());
    assert!(store.keys().all(|k| k.first() != Some(&ROOT)), "a v1 key starts with 0xF0");
    assert!(store[V1_METADATA_KEY].len() > ICON_BYTES);
    // v1's first bytes: 0x00 (balances), 0x01 (metadata), 'S' (STATE).
    let firsts: std::collections::BTreeSet<u8> = store.keys().map(|k| k[0]).collect();
    assert_eq!(firsts, [0x00, 0x01, b'S'].into());
}

/// Deploy v2 over v1, `migrate`, and check what was rewritten: STATE removed, one root record
/// written, every v1 key untouched.
#[test]
fn migrate_moves_only_the_root() {
    let before = populate_v1();
    let log = v2_init::<v2::Contract>(false, v2::Contract::migrate);
    println!("migrate body:\n{}\nflush:\n{}\ndrop:\n{}", fmt_ops(&log.body), fmt_ops(&log.flush), fmt_ops(&log.drop));
    // body: the init guard's has_key on the root, read STATE, remove STATE; flush: write the
    // root record. Nothing else.
    assert_eq!(log.rwd(), (1, 1, 1));
    assert_eq!((log.body[0].kind, log.body[0].key.as_slice()), (OpKind::HasKey, [ROOT].as_slice()));
    assert_eq!(log.body[1].key, b"STATE");
    assert_eq!(log.flush.len(), 1);
    assert_eq!(log.flush[0].key, vec![ROOT]);
    assert!(log.drop.is_empty());

    let after = testing::env_store();
    assert!(!after.contains_key(b"STATE".as_slice()));
    for (k, v) in before.iter().filter(|(k, _)| k.as_slice() != b"STATE") {
        assert_eq!(after.get(k), Some(v), "v1 key {k:02x?} changed");
    }
    assert_eq!(after.len(), before.len(), "STATE out, root in");

    // The root record: tag 1 = borsh(FungibleToken), tag 2 = borsh(LazyOption); tag 3 (a
    // collection) writes nothing inline.
    let fields = near_sdk::state::record::parse(&after[&vec![ROOT]]).unwrap();
    let tags: Vec<u32> = fields.iter().map(|(t, _)| *t).collect();
    assert_eq!(tags, vec![1, 2]);
    println!("root record: {} bytes; STATE was {} bytes", after[&vec![ROOT]].len(), before[b"STATE".as_slice()].len());

    // `migrate` cannot run twice: the v2 root now exists.
    let again = std::panic::catch_unwind(|| v2_init::<v2::Contract>(false, v2::Contract::migrate));
    assert!(again.is_err());
}

#[test]
fn every_old_balance_reads_back_through_the_v2_root() {
    populate_v1();
    v2_init::<v2::Contract>(false, v2::Contract::migrate);
    for i in 0..USERS {
        let (b, r) = v2_view::<v2::Contract, _>(|c| c.ft_balance_of(user(i)).0);
        assert_eq!(b, expected(i));
        // root read + the balance read; the 100 KiB metadata is never read.
        assert_eq!(r.reads(), 2);
        assert!(r.ops.iter().all(|o| o.key != V1_METADATA_KEY));
    }
    let (owner, _) = v2_view::<v2::Contract, _>(|c| c.ft_balance_of(acc(OWNER)).0);
    assert_eq!(owner, SUPPLY - transferred());
    let (supply, _) = v2_view::<v2::Contract, _>(|c| c.ft_total_supply().0);
    assert_eq!(supply, SUPPLY);
    let (meta, r) = v2_view::<v2::Contract, _>(|c| c.metadata.get().unwrap());
    assert_eq!(meta.icon.unwrap().len(), ICON_BYTES + "data:image/svg+xml,".len());
    assert!(r.read_bytes() > ICON_BYTES);
}

/// `ft_transfer` between two pre-existing accounts. `FungibleToken` holds
/// `collections::LookupMap` (write-through, no cache), so its writes land in the method body,
/// *before* the v2 flush, not after it. The new v2 field writes in the flush. The root is not
/// rewritten (no root scalar changed).
#[test]
fn transfer_between_old_accounts_after_migration() {
    populate_v1();
    v2_init::<v2::Contract>(false, v2::Contract::migrate);
    let (a, b) = (user(1), user(2));

    // v1 reference cost of the same transfer (before migration, on a fresh population).
    let (_, log) = v2_call::<v2::Contract, _>(a.as_str(), NearToken::from_yoctonear(1), |c| {
        c.ft_transfer(b.clone(), U128(10), None)
    });
    println!("v2 ft_transfer, first by this sender\nbody:\n{}\nflush:\n{}\ndrop:\n{}", fmt_ops(&log.body), fmt_ops(&log.flush), fmt_ops(&log.drop));

    // body: root read; v1 read+write sender, read+write receiver (write-through); then
    // `transfers_sent.entry(a).or_default()`: index read, header read, entry read? (see below)
    let (ka, kb) = (v1_balance_key(&a), v1_balance_key(&b));
    let body_keys: Vec<(OpKind, &[u8])> = log.body.iter().map(|o| (o.kind, o.key.as_slice())).collect();
    assert_eq!(body_keys[0], (OpKind::Read, [ROOT].as_slice()), "root read first");
    let v1_ops: Vec<(OpKind, &[u8])> =
        body_keys.iter().copied().filter(|(_, k)| k[0] == 0x00).collect();
    assert_eq!(
        v1_ops,
        vec![(OpKind::Read, &ka[..]), (OpKind::Write, &ka[..]), (OpKind::Read, &kb[..]), (OpKind::Write, &kb[..])],
        "v1 collections write through, inside the body"
    );
    // flush: only v2 keys (the new field's entry, index, slot, header), and no root write.
    assert!(log.flush.iter().all(|o| o.kind == OpKind::Write && o.key[0] == ROOT));
    assert_eq!(log.flush.len(), 4, "Map insert of a new key: 4 writes");
    assert!(log.flush.iter().all(|o| o.key != vec![ROOT]), "root unchanged, not rewritten");
    assert!(log.drop.is_empty(), "collections:: has no write-on-drop");
    assert!(log.all().iter().all(|o| o.key != V1_METADATA_KEY), "metadata untouched");
    assert!(log.all().iter().all(|o| o.value_len < 1024));

    let (bal, _) = v2_view::<v2::Contract, _>(|c| (c.ft_balance_of(a.clone()).0, c.ft_balance_of(b.clone()).0));
    assert_eq!(bal, (expected(1) - 10, expected(2) + 10));
    let (n, _) = v2_view::<v2::Contract, _>(|c| c.transfers_sent_by(a.clone()));
    assert_eq!(n, 1);

    // Second transfer by the same sender: the v2 field is an update (1 write).
    let (_, log2) = v2_call::<v2::Contract, _>(a.as_str(), NearToken::from_yoctonear(1), |c| {
        c.ft_transfer(b.clone(), U128(1), None)
    });
    println!("v2 ft_transfer, second: rwd {:?}\n{}", log2.rwd(), fmt_ops(&log2.all().into_iter().cloned().collect::<Vec<_>>()));
    assert_eq!(log2.flush.len(), 1);

    // Keys never overlap: v1 keys keep their first bytes, every new key starts with 0xF0.
    let store = testing::env_store();
    let firsts: std::collections::BTreeSet<u8> = store.keys().map(|k| k[0]).collect();
    assert_eq!(firsts, [0x00, 0x01, ROOT].into());
    assert_eq!(store.keys().filter(|k| k[0] == 0x00).count(), USERS + 1);
}

/// The same transfer under the unmodified v1 contract, for comparison.
#[test]
fn v1_transfer_cost_for_reference() {
    populate_v1();
    let (_, r) = v1_call(user(1).as_str(), NearToken::from_yoctonear(1), |c| c.ft_transfer(user(2), U128(10), None));
    println!("v1 ft_transfer: rwd {:?}, read bytes {}, written+replaced {}\n{}", r.rwd(), r.read_bytes(), r.written_and_replaced_bytes(), fmt_ops(&r.ops));
    // STATE read, 2 balance reads + writes, STATE write.
    assert_eq!(r.rwd(), (3, 3, 0));
}

/// New accounts registered after the upgrade still go into the v1 map (FungibleToken's), with
/// the v1 bond. Nothing about "add new fields as v2 types" helps the map that grows.
#[test]
fn new_registrations_after_the_upgrade_still_grow_the_v1_map() {
    populate_v1();
    v2_init::<v2::Contract>(false, v2::Contract::migrate);
    let (bond, _) = v2_view::<v2::Contract, _>(|c| c.storage_balance_bounds().min);
    let (_, log) = v2_call::<v2::Contract, _>("newbie.near", bond, |c| { c.storage_deposit(None, None); });
    let w: Vec<_> = log.all().into_iter().filter(|o| o.kind == OpKind::Write).collect();
    assert_eq!(w.len(), 1);
    assert_eq!(w[0].key, v1_balance_key(&acc("newbie.near")));
}

/// Store `store::` collections: `examples/status-message`'s v1 root holds a `store::LookupMap`
/// with an empty prefix (`RecordsKey` is a unit struct). Its write-on-drop runs after the v2
/// flush, root included.
mod status_message {
    use super::*;
    use near_sdk::store::LookupMap as V1LookupMap;
    use near_sdk::{BorshStorageKey, near};

    // examples/status-message/src/lib.rs, the state part, verbatim.
    #[derive(BorshStorageKey)]
    #[near]
    struct RecordsKey;

    #[near(contract_state)]
    pub struct StatusMessage {
        pub records: V1LookupMap<AccountId, String>,
    }
    impl Default for StatusMessage {
        fn default() -> Self {
            Self { records: V1LookupMap::new(RecordsKey) }
        }
    }

    pub use v2s::StatusV2;
    mod v2s {
    use super::*;
    #[near(contract_state(layout = v2))]
    pub struct StatusV2 {
        #[key(1, required)]
        pub records: V1LookupMap<AccountId, String>,
        #[key(2)]
        updates: near_sdk::state::LookupMap<AccountId, u32>,
        #[key(3)]
        last_writer: Option<AccountId>,
    }
    impl StatusV2 {
        pub fn migrate() -> Self {
            let old: StatusMessage = env::state_read().unwrap();
            env::storage_remove(b"STATE");
            Self { records: old.records, updates: Default::default(), last_writer: None }
        }
        pub fn set_status(&mut self, message: String) {
            let who = env::signer_account_id();
            self.records.insert(who.clone(), message);
            *self.updates.entry(who.clone()).or_default() += 1;
            self.last_writer = Some(who);
        }
    }
    }

    #[test]
    fn store_write_on_drop_lands_after_the_v2_flush() {
        ctx("alice.near", NearToken::from_yoctonear(0));
        testing::use_env_host();
        testing::env_log_start();
        let mut c = StatusMessage::default();
        for i in 0..50 {
            ctx(&format!("u{i}.near"), NearToken::from_yoctonear(0));
            c.records.insert(env::signer_account_id(), format!("hello {i}"));
        }
        env::state_write(&c);
        drop(c);
        let before = testing::env_store();
        // Empty prefix: keys are borsh(AccountId), first byte = id length (<= 64).
        assert!(before.keys().all(|k| k.first() != Some(&ROOT)));
        assert!(before.keys().filter(|k| k.as_slice() != b"STATE").all(|k| k[0] <= 64));

        v2_init::<StatusV2>(false, StatusV2::migrate);
        let (_, log) = v2_call::<StatusV2, _>("u7.near", NearToken::from_yoctonear(0), |c| {
            c.set_status("upgraded".into())
        });
        println!("status set_status after upgrade\nbody:\n{}\nflush:\n{}\ndrop:\n{}", fmt_ops(&log.body), fmt_ops(&log.flush), fmt_ops(&log.drop));
        // body: root read, updates entry read. flush: updates entry, then the root (scalar
        // changed), root last. drop: the v1 record write, after the root.
        assert_eq!(log.flush.last().unwrap().key, vec![ROOT]);
        assert_eq!(log.drop.len(), 1);
        assert_eq!(log.drop[0].kind, OpKind::Write);
        let mut v1k = Vec::new();
        v1k.extend(near_sdk::borsh::to_vec(&acc("u7.near")).unwrap());
        assert_eq!(log.drop[0].key, v1k);
        let (s, _) = v2_view::<StatusV2, _>(|c| c.records.get(&acc("u7.near")).cloned());
        assert_eq!(s.as_deref(), Some("upgraded"));
    }

    /// A v1 `store` write still pending when the method panics after the v2 flush... cannot
    /// happen in the wrapper (nothing runs between flush and drop). But a view method that
    /// reads a `store::LookupMap` caches nothing written, so views stay clean.
    #[test]
    fn view_over_store_field_is_clean() {
        ctx("alice.near", NearToken::from_yoctonear(0));
        testing::use_env_host();
        testing::env_log_start();
        let mut c = StatusMessage::default();
        c.records.insert(acc("bob.near"), "hi".into());
        env::state_write(&c);
        drop(c);
        v2_init::<StatusV2>(false, StatusV2::migrate);
        let (s, r) = v2_view::<StatusV2, _>(|c| c.records.get(&acc("bob.near")).cloned());
        assert_eq!(s.as_deref(), Some("hi"));
        assert_eq!(r.writes(), 0);
    }
}

// ---------------------------------------------------------------------------------------
// The init window between deploy and migrate
// ---------------------------------------------------------------------------------------

/// Between "deploy v2" and "call migrate", the v2 root does not exist, so the `#[init]`
/// guard passes for *any* `#[init]` method of the v2 contract. v2 here has only the private
/// `migrate`; a contract that kept a public `#[init] new` without the example's explicit
/// `require!(!env::state_exists())` could be re-initialized by anyone in that window.
#[test]
fn v2_root_is_absent_until_migrate_runs() {
    populate_v1();
    ctx(CONTRACT, NearToken::from_yoctonear(0));
    testing::begin_call();
    assert!(!<v2::Contract as ContractState>::state_exists());
    assert!(env::state_exists(), "the v1 STATE blob is what still exists");
    // A non-init call in the window: `load_root` decodes an empty record, and the `required`
    // v1 fields fail.
    let r = std::panic::catch_unwind(|| {
        v2_view::<v2::Contract, _>(|c| c.ft_total_supply().0)
    });
    assert!(r.is_err());
}

// ---------------------------------------------------------------------------------------
// Claim 3: Migrating
// ---------------------------------------------------------------------------------------

fn upgrade_to_v3() {
    populate_v1();
    v2_init::<v2::Contract>(false, v2::Contract::migrate);
    // one transfer under v2 so `transfers_sent` has an entry to carry over
    v2_call::<v2::Contract, _>(user(9).as_str(), NearToken::from_yoctonear(1), |c| {
        c.ft_transfer(user(8), U128(1), None)
    });
    let log = v2_init::<v3::Contract>(true, v3::Contract::migrate);
    println!("v3 migrate: rwd {:?}\n{}", log.rwd(), fmt_ops(&log.all().into_iter().cloned().collect::<Vec<_>>()));
    // root read, root write; no balance moves.
    assert_eq!(log.rwd(), (1, 1, 0));
}

#[test]
fn v3_migrate_rewrites_the_root_only_and_drops_the_retired_tag() {
    upgrade_to_v3();
    let store = testing::env_store();
    let tags: Vec<u32> = near_sdk::state::record::parse(&store[&vec![ROOT]]).unwrap().iter().map(|(t, _)| *t).collect();
    assert_eq!(tags, vec![2, 4, 5, 6], "tag 1 (reserved) dropped; 4 holds the v1 prefix inline");
    let (n, _) = v2_view::<v3::Contract, _>(|c| c.transfers_sent_by(user(9)));
    assert_eq!(n, 1, "the v2 field at tag 3 carried over untouched");
    let (s, _) = v2_view::<v3::Contract, _>(|c| c.ft_total_supply().0);
    assert_eq!(s, SUPPLY);
    // A second migrate finds no tag 1.
    let again = std::panic::catch_unwind(|| v2_init::<v3::Contract>(true, v3::Contract::migrate));
    assert!(again.is_err());
}

/// Read fallback: an old account costs two reads below the root (new miss, old hit); a key on
/// neither side costs two misses.
#[test]
fn migrating_read_falls_back_with_two_reads_per_miss() {
    upgrade_to_v3();
    let (b, r) = v2_view::<v3::Contract, _>(|c| c.ft_balance_of(user(5)).0);
    assert_eq!(b, expected(5));
    let below_root: Vec<_> = r.ops.iter().filter(|o| o.key != vec![ROOT]).collect();
    println!("old-account read: {}", fmt_ops(&r.ops));
    assert_eq!(below_root.len(), 2);
    assert_eq!(below_root[0].key[0], ROOT, "new side first");
    assert_eq!(below_root[0].value_len, 0, "miss");
    assert_eq!(below_root[1].key, v1_balance_key(&user(5)), "then the old side");

    let (b, r) = v2_view::<v3::Contract, _>(|c| c.ft_balance_of(acc("nobody.near")).0);
    assert_eq!(b, 0);
    assert_eq!(r.reads(), 3, "root + new miss + old miss");
}

/// Write forward: the first transfer touching two old accounts moves both (v2 write at flush,
/// v1 remove in the body); afterwards they cost one read each.
#[test]
fn migrating_write_moves_the_key_and_deletes_the_old_entry() {
    upgrade_to_v3();
    let (a, b) = (user(1), user(2));
    let (_, log) = v2_call::<v3::Contract, _>(a.as_str(), NearToken::from_yoctonear(1), |c| {
        c.ft_transfer(b.clone(), U128(10), None)
    });
    println!("v3 ft_transfer, both accounts on the old side: rwd {:?}\nbody:\n{}\nflush:\n{}", log.rwd(), fmt_ops(&log.body), fmt_ops(&log.flush));
    let store = testing::env_store();
    assert!(!store.contains_key(&v1_balance_key(&a)), "old entry deleted");
    assert!(!store.contains_key(&v1_balance_key(&b)));
    let removes: Vec<_> = log.body.iter().filter(|o| o.kind == OpKind::Remove).collect();
    assert_eq!(removes.len(), 2);
    // At flush: 2 balance writes on the v2 side + 4 for the sender's new transfers_sent key.
    assert_eq!(log.flush.len(), 2 + 4);
    assert_eq!(log.rwd(), (7, 6, 2));

    let (bal, r) = v2_view::<v3::Contract, _>(|c| c.ft_balance_of(a.clone()).0);
    assert_eq!(bal, expected(1) - 10);
    assert_eq!(r.reads(), 2, "root + new-side hit; the old side is not consulted");

    // Second transfer between the same, now-migrated accounts.
    let (_, log2) = v2_call::<v3::Contract, _>(a.as_str(), NearToken::from_yoctonear(1), |c| {
        c.ft_transfer(b.clone(), U128(1), None)
    });
    println!("v3 ft_transfer, both migrated: rwd {:?}", log2.rwd());
    assert_eq!(log2.count(OpKind::Remove), 0);

    // Storage: each moved account's key grew.
    let new_keys: Vec<_> = store.keys().filter(|k| k[0] == ROOT && k.len() > 1 && k[1] == 4).collect();
    assert_eq!(new_keys.len(), 2);
    let old_len = v1_balance_key(&a).len();
    println!("v1 key {old_len} bytes -> v2 key {} bytes", new_keys[0].len());
    assert_eq!(new_keys[0].len(), old_len + 18);
}

/// The v1 bond (`account_storage_usage`, measured by v1 for v1 key lengths) undercharges a
/// registration that lands on the v2 side.
#[test]
fn v1_storage_bond_undercharges_v2_keys() {
    upgrade_to_v3();
    let (bond_bytes, _) = v2_view::<v3::Contract, _>(|c| c.account_storage_usage);
    let id = "a".repeat(59) + ".near"; // 64 chars, the longest id
    let bond = env::storage_byte_cost().saturating_mul(bond_bytes.into());
    ctx(&id, bond);
    let before = env::storage_usage();
    let (_, _log) = v2_call::<v3::Contract, _>(&id, bond, |c| c.storage_deposit(None));
    let used = env::storage_usage() - before;
    println!("v1 bond {bond_bytes} bytes; a v2-side registration used {used} bytes");
    assert!(used > bond_bytes, "underpaid by {} bytes", used - bond_bytes);
}
