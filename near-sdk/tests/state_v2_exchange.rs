//! The spec's `Exchange` contract, written with the real macros (`#[near(storage)]`,
//! `#[key(n)]`, `#[near(contract_state(layout = v2))]`) and driven the way the `#[near]`
//! wrapper drives it: `ContractState::state_read()`, the method, `state_write()`.
//! (The wasm wrapper itself is `cfg(target_arch = "wasm32")`, so it is not compiled here.)
#![cfg(feature = "unstable")]

use near_sdk::json_types::U128;
use near_sdk::state::testing::{self, Receipt};
use near_sdk::state::{ContractState, GroupedMap, IterMap, Lazy, Map, Page};
use near_sdk::{AccountId, env, near};

pub type TokenId = String;
pub type MarketId = u32;
pub type OrderId = u64;

// Revision 1: no `removable_leaf!` needed, every Borsh leaf is Removable.
#[near(serializers = [borsh, json])]
#[derive(Clone, Debug, PartialEq)]
pub struct Order {
    pub price: u128,
    pub qty: u128,
}

#[near(serializers = [borsh])]
#[derive(Default)]
pub struct Metadata {
    pub name: String,
    pub blob: Vec<u8>,
}

// `owner: AccountId` has no `Default`: `required` is the one edit to the spec's struct.
// Revision 1: no `PanicOnDefault` (the derive emits `Default`), no `#[derive(Default)]` on Account.
#[near(contract_state(layout = v2))]
pub struct Exchange {
    #[key(1, required)]
    owner: AccountId,
    #[key(2)]
    fee_bps: u16,
    #[key(3)]
    accounts: Map<AccountId, Account>,
    #[key(4)]
    orders: GroupedMap<MarketId, OrderId, Order>,
    #[key(5)]
    metadata: Lazy<Metadata>, // large, loaded only when touched
}

#[near(storage)]
pub struct Account {
    #[key(1)]
    nonce: u64,
    #[key(2)]
    balances: IterMap<TokenId, u128>, // lives at 3/<account>/2/<token>
}

#[near]
impl Exchange {
    #[init]
    pub fn new(owner: AccountId) -> Self {
        Self {
            owner,
            fee_bps: 0,
            accounts: Default::default(),
            orders: Default::default(),
            metadata: Default::default(),
        }
    }

    /// Item 12: an init body that seeds collections binds the root first.
    #[init]
    pub fn new_seeded(owner: AccountId) -> Self {
        let mut this = Self {
            owner: owner.clone(),
            fee_bps: 0,
            accounts: Default::default(),
            orders: Default::default(),
            metadata: Default::default(),
        }
        .bind();
        this.accounts.entry(owner).or_default().nonce = 1;
        this
    }

    // ---- the spec's four methods, verbatim except for `place` and `set_fee` ----

    pub fn deposit(&mut self, token: TokenId, amount: U128) {
        let acct = self.accounts.entry(env::predecessor_account_id()).or_default();
        *acct.balances.entry(token).or_default() += amount.0;
    }

    // one storage read; the Account record is never loaded
    pub fn balance_of(&self, id: AccountId, token: TokenId) -> U128 {
        self.accounts.at(&id).balances.get(&token).copied().unwrap_or(0).into()
    }

    // cursor pagination; offset pagination stays available and cheap
    pub fn balances(&self, id: AccountId, after: Option<TokenId>) -> Page<TokenId, u128> {
        self.accounts.at(&id).balances.page(after.as_ref(), 50)
    }

    pub fn cancel(&mut self, market: MarketId, order: OrderId) {
        self.orders.group_mut(&market).remove(&order);
    }

    // ---- extra methods for the tests ----

    pub fn place(&mut self, market: MarketId, id: OrderId, order: Order) {
        self.orders.group_mut(&market).insert(id, order);
    }

    // Revision 2 item 1: the reference outlives the `group_mut` temporary (E0716 before).
    pub fn fill(&mut self, market: MarketId, order: OrderId, qty: U128) -> U128 {
        let o = self.orders.group_mut(&market).get_mut(&order).expect("no such order");
        o.qty -= qty.0;
        o.qty.into()
    }

    pub fn set_fee(&mut self, fee_bps: u16) {
        assert_eq!(env::predecessor_account_id(), self.owner, "owner only");
        self.fee_bps = fee_bps;
    }

    pub fn bump_nonce(&mut self) -> u64 {
        let acct = self.accounts.entry(env::predecessor_account_id()).or_default();
        acct.nonce += 1;
        acct.nonce
    }

    pub fn set_metadata(&mut self, name: String, blob_len: u32) {
        self.metadata.set(Metadata { name, blob: vec![1; blob_len as usize] });
    }
}

// ---------------------------------------------------------------------------------------
// harness: what the wrapper does around each method
// ---------------------------------------------------------------------------------------

fn as_caller(id: &str) {
    let ctx = near_sdk::test_utils::VMContextBuilder::new()
        .predecessor_account_id(id.parse().unwrap())
        .build();
    near_sdk::testing_env!(ctx);
}

fn init(owner: &str) -> Receipt {
    testing::use_mock();
    let c = Exchange::new(owner.parse().unwrap());
    c.state_write();
    testing::receipt()
}

fn call<R>(who: &str, f: impl FnOnce(&mut Exchange) -> R) -> (R, Receipt) {
    as_caller(who);
    testing::begin_call();
    let mut c = Exchange::state_read().expect("v2 state_read never returns None");
    let r = f(&mut c);
    c.state_write();
    (r, testing::receipt())
}

fn view<R>(f: impl FnOnce(&Exchange) -> R) -> (R, Receipt) {
    testing::begin_call();
    let c = Exchange::state_read().unwrap();
    let r = f(&c);
    testing::assert_view_clean();
    (r, testing::receipt())
}

fn alice() -> AccountId {
    "alice.near".parse().unwrap()
}

// ---------------------------------------------------------------------------------------

#[test]
fn init_writes_one_record() {
    let r = init("owner.near");
    assert_eq!(r.rwd(), (0, 1, 0));
    assert_eq!(r.ops[0].key, vec![0xF0]);
    assert_eq!(Exchange::state_key(), &[0xF0]);
}

#[test]
fn deposit_balance_of_and_pagination() {
    init("owner.near");
    let (_, r) = call("alice.near", |c| c.deposit("usdt".into(), U128(5)));
    assert_eq!(
        r.rwd(),
        (4, 5, 0),
        "root, account, usdt entry, header reads; account record + entry, index, slot, header writes"
    );
    let (_, r) = call("alice.near", |c| c.deposit("usdt".into(), U128(5)));
    assert_eq!(r.rwd(), (3, 1, 0));

    let (bal, r) = view(|c| c.balance_of(alice(), "usdt".into()));
    assert_eq!(bal.0, 10);
    assert_eq!(
        r.reads(),
        2,
        "the spec says 'one storage read'; the wrapper's root read makes it two"
    );

    for i in 0..120 {
        call("alice.near", |c| c.deposit(format!("t{i:03}"), U128(1)));
    }
    let (p1, r) = view(|c| c.balances(alice(), None));
    assert_eq!(p1.items.len(), 50);
    assert_eq!(p1.items[0], ("usdt".to_string(), 10));
    assert_eq!(r.reads(), 1 + 1 + 2 * 50, "root + header + 2 per item");
    let (p2, r) = view(|c| c.balances(alice(), p1.next.clone()));
    assert_eq!(r.reads(), 1 + 2 + 2 * 50, "root + cursor index + header + 2 per item");
    let (p3, _) = view(|c| c.balances(alice(), p2.next.clone()));
    assert_eq!(p3.items.len(), 21);
    assert_eq!(p3.next, None);
    // Page serializes to JSON as the spec says
    let json = near_sdk::serde_json::to_string(&p3).unwrap();
    assert!(json.starts_with("{\"items\":[[\"t"));
}

#[test]
fn cancel_through_grouped_map() {
    init("owner.near");
    for id in 0..3 {
        call("alice.near", |c| c.place(7, id, Order { price: 1, qty: id as u128 }));
    }
    let (_, r) = call("alice.near", |c| c.cancel(7, 0));
    // root; entry, index, header, last slot reads; moved slot + index + header writes; 3 removes
    assert_eq!(r.rwd(), (5, 3, 3));
    let (_, r) = call("alice.near", |c| c.cancel(7, 99));
    assert_eq!(r.rwd(), (2, 0, 0), "cancelling an unknown order: root + entry read");
}

/// Revision 2 item 1: `group_mut(&m).get_mut(&k)` bound to a `let`, and a named group handle
/// used for several operations, compile and cost what the plain `IterMap` costs.
#[test]
fn group_mut_references_outlive_the_temporary() {
    init("owner.near");
    call("alice.near", |c| {
        c.place(7, 1, Order { price: 1, qty: 10 });
        c.place(7, 2, Order { price: 1, qty: 20 });
    });
    let (left, r) = call("alice.near", |c| c.fill(7, 1, U128(4)));
    assert_eq!(left.0, 6);
    assert_eq!(r.rwd(), (2, 1, 0), "root + order read, order write");

    let (sum, _) = call("alice.near", |c| {
        let g = c.orders.group_mut(&7);
        g.get_mut(&1).unwrap().qty += 1;
        let b = g.get_mut(&2).unwrap();
        b.qty += 1;
        let total = b.qty + g.get(&1).unwrap().qty;
        g.insert(3, Order { price: 2, qty: 0 });
        total
    });
    assert_eq!(sum, 21 + 7);
    let (n, _) = view(|c| c.orders.group(&7).len());
    assert_eq!(n, 3);
}

/// Revision 2 item 3: a key over its declared `MAX_LEN` panics at the map hop, naming the
/// limit. The limit is in borsh bytes: a `String` key may hold 512 bytes of text, which the
/// message reports as 516 (the 4-byte length prefix included).
#[test]
#[should_panic(expected = "storage key is 604 bytes, over its declared MAX_LEN of 516")]
fn oversized_key_panics_naming_the_limit() {
    init("owner.near");
    call("alice.near", |c| {
        let acct = c.accounts.entry(alice()).or_default();
        acct.balances.insert("x".repeat(600), 1);
    });
}

#[test]
fn root_scalar_change_rewrites_root_only_when_changed() {
    init("owner.near");
    let (_, r) = call("owner.near", |c| c.set_fee(30));
    assert_eq!(r.rwd(), (1, 1, 0));
    let (_, r) = call("owner.near", |c| c.set_fee(30));
    assert_eq!(r.rwd(), (1, 0, 0), "same value: compare at flush, no write");
}

#[test]
fn lazy_metadata_is_not_read_unless_touched() {
    init("owner.near");
    call("owner.near", |c| c.set_metadata("x".into(), 100 * 1024));
    let (_, r) = call("alice.near", |c| c.bump_nonce());
    assert!(r.max_value_bytes() < 1024, "the 100 KiB Lazy is never read by other methods");
}

/// A newer version wrote root tag 9. The current code rewrites the root and keeps it.
#[test]
fn unknown_root_tag_survives_rollback() {
    init("owner.near");
    let mut root = testing::store()[&vec![0xF0]].clone();
    root.extend_from_slice(&[0x09, 0x02, 0xAA, 0xBB]); // tag 9 > every known tag, so still sorted
    testing::seed(&[0xF0], &root);
    call("owner.near", |c| c.set_fee(5));
    let stored = testing::store()[&vec![0xF0]].clone();
    assert!(stored.ends_with(&[0x09, 0x02, 0xAA, 0xBB]));
}

#[test]
#[should_panic(expected = "Missing { tag: 1 }")]
fn uninitialized_contract_panics_on_required_field() {
    testing::use_mock();
    view(|c| c.balance_of(alice(), "usdt".into()));
}

/// Without `.bind()`, an init body that touches a collection still panics, now pointing at it.
#[test]
#[should_panic(expected = "call `.bind()` on the root")]
fn init_body_without_bind_panics_with_hint() {
    testing::use_mock();
    let mut c = Exchange::new("owner.near".parse().unwrap());
    c.accounts.insert(alice(), Account::default());
}

/// Item 12: `.bind()` in `#[init]` lets the body seed collections; the wrapper's write
/// flushes them before the root.
#[test]
fn init_with_bind_seeds_collections() {
    testing::use_mock();
    let c = Exchange::new_seeded("owner.near".parse().unwrap());
    c.state_write();
    let r = testing::receipt();
    // has_key? no: or_default reads the record (1 read), then record write + root write
    assert_eq!(r.rwd(), (1, 2, 0));
    assert_eq!(r.ops.last().unwrap().key, vec![0xF0], "root last");
    let (n, _) = view(|c| c.accounts.get(&"owner.near".parse().unwrap()).map(|a| a.nonce));
    assert_eq!(n, Some(1));
}

/// `bind()` is idempotent.
#[test]
fn bind_is_idempotent() {
    testing::use_mock();
    let c = Exchange::new("owner.near".parse().unwrap()).bind().bind();
    assert!(c.accounts.get(&alice()).is_none());
}

/// Item 13: `state_exists()` on a v2 root goes through the host trait (round 1: it bypassed
/// it and saw nothing in the counting mock).
#[test]
fn state_exists_goes_through_the_host() {
    init("owner.near");
    testing::begin_call();
    assert!(Exchange::state_exists());
    assert_eq!(testing::receipt().has_keys(), 1);
}

// ---------------------------------------------------------------------------------------
// Item 5: a `required` field added after deploy fails every old record
// ---------------------------------------------------------------------------------------

#[near(storage)]
pub struct SlimV1 {
    #[key(1)]
    nonce: u64,
}

#[near(storage)]
pub struct SlimV2 {
    #[key(1)]
    nonce: u64,
    #[key(2, required)]
    owner: AccountId, // added in v2: wrong, must be Option<AccountId>
}

#[test]
#[should_panic(expected = "Missing { tag: 2 }")]
fn required_field_added_later_fails_old_records() {
    use near_sdk::state::Path;
    testing::use_mock();
    let root = Path::root(0xF0).field(9);
    let mut v1: Map<u32, SlimV1> = Map::__bound_at(root.clone());
    v1.insert(1, SlimV1 { nonce: 3 });
    testing::flush();
    let v2: Map<u32, SlimV2> = Map::__bound_at(root);
    let _ = v2.get(&1).map(|s| (&s.owner, s.nonce));
}

// ---------------------------------------------------------------------------------------
// v1 collections as leaf fields (item 9)
// ---------------------------------------------------------------------------------------

mod legacy {
use super::*;

#[near(contract_state(layout = v2, root = 0xF1))]
pub struct Legacy {
    #[key(1, required)]
    old: near_sdk::store::LookupMap<u32, u32>,
    #[key(2)]
    users: Map<u32, User>,
}

// `notes: near_sdk::store::LookupMap<u32, u32>` is now rejected by the derive ("v1 collection
// `LookupMap` inside a storage record"). A type alias slips past the name check: this is the
// "best effort" hole, used here on purpose to exercise the drop-after-flush safety net.
type Notes = near_sdk::store::LookupMap<u32, u32>;

#[near(storage)]
pub struct User {
    // LookupMap has no Default, so the field must be `required`.
    #[key(1, required)]
    notes: Notes,
}

fn v1_key(prefix: &[u8], k: u32) -> Vec<u8> {
    let mut key = prefix.to_vec();
    key.extend_from_slice(&k.to_le_bytes());
    key
}

fn setup() {
    as_caller("alice.near");
    testing::use_env_host();
    let c = Legacy { old: near_sdk::store::LookupMap::new(b"o".to_vec()), users: Default::default() };
    c.state_write();
    drop(c);
}

/// At the root, a v1 field is dropped with the root after `state_write`, so its writes land
/// after the v2 flush (not "root last"), but they do land.
#[test]
fn v1_leaf_at_root_flushes_on_drop_after_the_v2_flush() {
    setup();
    testing::begin_call();
    let mut c = Legacy::state_read().unwrap();
    c.old.insert(1, 11);
    c.state_write();
    let k = v1_key(b"o", 1);
    assert!(!env::storage_has_key(&k), "nothing written by the v2 flush");
    drop(c);
    assert!(env::storage_has_key(&k), "written by LookupMap's Drop, after the root");
}

/// Item 9: nested in a record held by the v2 cache, a v1 field's write-on-drop now runs when
/// the flush frees the cache slots: late (after the root) but not lost. Round 1: lost.
#[test]
fn v1_leaf_nested_in_a_cached_record_writes_late_not_lost() {
    setup();
    testing::begin_call();
    let mut c = Legacy::state_read().unwrap();
    c.users.insert(1, User { notes: near_sdk::store::LookupMap::new(b"n1".to_vec()) });
    c.state_write();
    drop(c);

    testing::begin_call();
    let mut c = Legacy::state_read().unwrap();
    c.users.get_mut(&1).unwrap().notes.insert(5, 55);
    let k = v1_key(b"n1", 5);
    assert!(!env::storage_has_key(&k));
    c.state_write();
    assert!(env::storage_has_key(&k), "written when the flush dropped the cached record");
    drop(c);
}

/// Against revision item 6 ("no leaf can orphan anything"): a nested v1 collection that slips
/// past the name check is Borsh, hence blanket-`Removable`, so `remove` compiles and leaves
/// the v1 entries behind with nothing pointing at them.
#[test]
fn removing_a_record_with_a_nested_v1_field_orphans_its_entries() {
    setup();
    testing::begin_call();
    let mut c = Legacy::state_read().unwrap();
    let mut u = User { notes: near_sdk::store::LookupMap::new(b"n2".to_vec()) };
    u.notes.insert(7, 77);
    c.users.insert(2, u);
    c.state_write();
    drop(c);
    let k = v1_key(b"n2", 7);
    assert!(env::storage_has_key(&k));

    testing::begin_call();
    let mut c = Legacy::state_read().unwrap();
    assert!(c.users.remove(&2).is_some());
    c.state_write();
    drop(c);
    assert!(env::storage_has_key(&k), "orphaned: the record is gone, its v1 entry is not");
}

} // mod legacy


// ---------------------------------------------------------------------------------------
// reserved tags
// ---------------------------------------------------------------------------------------

#[near(storage, reserved(3))]
#[derive(Default)]
pub struct Slim {
    #[key(1)]
    nonce: u64,
}

#[test]
fn reserved_tag_is_dropped_on_rewrite_unknown_tag_is_kept() {
    use near_sdk::state::{Path, record};
    testing::use_mock();
    let root = Path::root(0xF0).field(9);
    let mut m: Map<u32, Slim> = Map::__bound_at(root.clone());
    let key = root.kind(0).entry(&1u32);
    let mut w = record::RecordWriter::new();
    w.push(1, borsh::to_vec(&1u64).unwrap());
    w.push(3, vec![0x33]); // a deliberately removed field
    w.push(4, vec![0x44]); // a field from a newer version
    testing::seed(key.as_bytes(), &w.finish(None, |_| true));
    m.get_mut(&1).unwrap().nonce = 2;
    testing::flush();
    let tags: Vec<u32> = record::parse(&testing::store()[key.as_bytes()])
        .unwrap()
        .into_iter()
        .map(|(t, _)| t)
        .collect();
    assert_eq!(tags, vec![1, 4]);
}

/// The same flow through the real mocked blockchain (`env::storage_*`), no counting mock.
#[test]
fn works_against_the_mocked_blockchain() {
    as_caller("alice.near");
    // no `use_mock()`: the default host is env
    near_sdk::state::testing::use_env_host();
    let c = Exchange::new("owner.near".parse().unwrap());
    c.state_write();
    assert!(Exchange::state_exists());
    let mut c = Exchange::state_read().unwrap();
    c.deposit("usdt".into(), U128(3));
    c.state_write();
    let c = Exchange::state_read().unwrap();
    assert_eq!(c.balance_of(alice(), "usdt".into()).0, 3);
    assert!(env::storage_has_key(&[0xF0]));
}
