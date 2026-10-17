//! The spec's `Exchange` contract, written with the real macros (`#[near(storage)]`,
//! `#[key(n)]`, `#[near(contract_state(layout = v2))]`) and driven the way the `#[near]`
//! wrapper drives it: `ContractState::state_read()`, the method, `state_write()`.
//! (The wasm wrapper itself is `cfg(target_arch = "wasm32")`, so it is not compiled here.)
#![cfg(feature = "unstable")]

use near_sdk::json_types::U128;
use near_sdk::state::testing::{self, Receipt};
use near_sdk::state::{ContractState, GroupedMap, IterMap, Lazy, Map, Page};
use near_sdk::{AccountId, PanicOnDefault, env, near};

pub type TokenId = String;
pub type MarketId = u32;
pub type OrderId = u64;

#[near(serializers = [borsh, json])]
#[derive(Clone, Debug, PartialEq)]
pub struct Order {
    pub price: u128,
    pub qty: u128,
}
// Without this, `self.orders.group_mut(&market).remove(&order)` does not compile:
// `remove` needs `Order: Removable`, and a plain Borsh leaf must opt in.
near_sdk::state::removable_leaf!(Order);

#[near(serializers = [borsh])]
#[derive(Default)]
pub struct Metadata {
    pub name: String,
    pub blob: Vec<u8>,
}

// `owner: AccountId` has no `Default`, so a missing tag cannot read as a default:
// the spec's `#[key(1)] owner: AccountId` does not compile; `required` makes it an error instead.
#[near(contract_state(layout = v2))]
#[derive(PanicOnDefault)]
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
#[derive(Default)]
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

/// The init body runs before the wrapper can bind the root's collections.
#[test]
#[should_panic(expected = "not bound")]
fn init_body_cannot_touch_collections() {
    testing::use_mock();
    let mut c = Exchange::new("owner.near".parse().unwrap());
    c.accounts.insert(alice(), Account::default());
}

/// `state_exists()` (the init-twice guard) goes straight to `env::storage_has_key`, bypassing
/// the cache and the `Host` trait: here the counting mock holds the root but env does not.
#[test]
fn state_exists_bypasses_the_host() {
    init("owner.near");
    assert!(testing::store().contains_key(&vec![0xF0]));
    assert!(!Exchange::state_exists());
}

// ---------------------------------------------------------------------------------------
// v1 collections as leaf fields ("keeps v1's write-on-drop")
// ---------------------------------------------------------------------------------------

mod legacy {
use super::*;

#[near(contract_state(layout = v2, root = 0xF1))]
#[derive(PanicOnDefault)]
pub struct Legacy {
    #[key(1, required)]
    old: near_sdk::store::LookupMap<u32, u32>,
    #[key(2)]
    users: Map<u32, User>,
}

#[near(storage)]
pub struct User {
    // LookupMap has no Default, so the field must be `required`.
    #[key(1, required)]
    notes: near_sdk::store::LookupMap<u32, u32>,
}

fn v1_key(prefix: &[u8], k: u32) -> Vec<u8> {
    let mut key = prefix.to_vec();
    key.extend_from_slice(&k.to_le_bytes());
    key
}

/// At the root, a v1 field is dropped with the root after `state_write`, so its writes land
/// after the v2 flush (not "root last"), but they do land.
#[test]
fn v1_leaf_at_root_flushes_on_drop_after_the_v2_flush() {
    as_caller("alice.near");
    testing::use_env_host();
    let c = Legacy {
        old: near_sdk::store::LookupMap::new(b"o".to_vec()),
        users: Default::default(),
    };
    c.state_write();
    drop(c);

    testing::begin_call();
    let mut c = Legacy::state_read().unwrap();
    c.old.insert(1, 11);
    c.state_write();
    let k = v1_key(b"o", 1);
    assert!(!env::storage_has_key(&k), "nothing written by the v2 flush");
    drop(c);
    assert!(env::storage_has_key(&k), "written by LookupMap's Drop, after the root");
}

/// Nested in a record that lives in the v2 cache, a v1 field is never dropped (cache values
/// are leaked per call), so its write-on-drop never happens: the write is lost silently.
#[test]
fn v1_leaf_nested_in_a_cached_record_never_flushes() {
    as_caller("alice.near");
    testing::use_env_host();
    let c = Legacy {
        old: near_sdk::store::LookupMap::new(b"o".to_vec()),
        users: Default::default(),
    };
    c.state_write();
    drop(c);

    testing::begin_call();
    let mut c = Legacy::state_read().unwrap();
    c.users.insert(1, User { notes: near_sdk::store::LookupMap::new(b"n1".to_vec()) });
    c.state_write();
    drop(c);

    testing::begin_call();
    let mut c = Legacy::state_read().unwrap();
    c.users.get_mut(&1).unwrap().notes.insert(5, 55);
    c.state_write();
    drop(c);
    testing::begin_call();

    let k = v1_key(b"n1", 5);
    assert!(!env::storage_has_key(&k), "the v1 write was lost");
}

} // mod legacy

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
