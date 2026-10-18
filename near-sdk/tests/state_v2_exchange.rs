//! The spec's `Exchange` contract, written with the real macros (`#[near(storage)]`,
//! `#[key(n)]`, `#[near(contract_state(layout = v2))]`) and driven the way the `#[near]`
//! wrapper drives it: `ContractState::state_read()`, the method, `state_write()`.
//! (The wasm wrapper itself is `cfg(target_arch = "wasm32")`, so it is not compiled here.)
#![cfg(feature = "unstable")]

use near_sdk::json_types::U128;
use near_sdk::state::testing::{self, Receipt};
use near_sdk::state::{ContractState, CursorGone, Lazy, LookupMap, Map, Page};
use near_sdk::{AccountId, env, near};

pub type TokenId = String;
pub type MarketId = u32;
pub type OrderId = u64;

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

// D3 (Oct 17): `GroupedMap` is deferred. The order books are `markets: Map<MarketId, Market>`,
// with `Market` a record holding its `orders` map and its scalars. The struct and the five
// spec methods (`deposit`, `set_balance`, `balance_of`, `balances`, `cancel`) are the shape the
// revised doc prints.
#[near(contract_state(layout = v2))]
pub struct Exchange {
    #[key(1, required)] owner: AccountId,   // no Default, so a missing tag is an error
    #[key(2)] fee_bps: u16,
    #[key(3)] accounts: Map<AccountId, Account>,
    #[key(4)] markets: Map<MarketId, Market>,
    #[key(5)] metadata: Lazy<Metadata>,          // large, loaded only when touched
}

#[near(storage)]
pub struct Account {
    #[key(1)] nonce: u64,
    #[key(2)] balances: Map<TokenId, u128>,      // lives at 3/<account>/2/<token>
}

#[near(storage)]
pub struct Market {
    #[key(1)] orders: Map<OrderId, Order>,       // lives at 4/<market>/1/<order>
    #[key(2)] tick_size: u128,
    #[key(3)] next_order_id: OrderId,
}

#[near]
impl Exchange {
    #[init]
    pub fn new(owner: AccountId) -> Self {
        Self {
            owner,
            fee_bps: 0,
            accounts: Default::default(),
            markets: Default::default(),
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
            markets: Default::default(),
            metadata: Default::default(),
        }
        .bind();
        this.accounts.entry(owner).or_default().nonce = 1;
        this
    }

    // ---- the spec's five methods ----

    pub fn deposit(&mut self, token: TokenId, amount: U128) {
        let acct = self.accounts.entry(env::predecessor_account_id()).or_default();
        *acct.balances.entry(token).or_default() += amount.0;
    }

    // writes below the record without reading it: one existence check, no decode
    pub fn set_balance(&mut self, id: AccountId, token: TokenId, amount: U128) {
        self.accounts.at_mut(&id).expect("no such account").balances.insert(token, amount.0);
    }

    // one read below the root; the Account record is never loaded
    pub fn balance_of(&self, id: AccountId, token: TokenId) -> U128 {
        self.accounts.at(&id).balances.get(&token).copied().unwrap_or(0).into()
    }

    // cursor pagination; a cursor removed since the last page is `Err(CursorGone)`
    #[handle_result]
    pub fn balances(
        &self,
        id: AccountId,
        after: Option<TokenId>,
    ) -> Result<Page<TokenId, u128>, CursorGone> {
        self.accounts.at(&id).balances.page(after.as_ref(), 50)
    }

    pub fn cancel(&mut self, market: MarketId, order: OrderId) {
        self.markets.at_mut(&market).expect("no such market").orders.remove(&order);
    }

    // ---- extra methods for the tests ----

    /// D6: a record that owns collections goes in with `insert_new` (plain `insert` does not
    /// compile for `Market`).
    pub fn open_market(&mut self, market: MarketId, tick_size: U128) {
        self.markets.insert_new(market, Market { tick_size: tick_size.0, ..Default::default() });
    }

    /// Scalars and collections of one record together: `get_mut` reads the record.
    pub fn place(&mut self, market: MarketId, order: Order) -> OrderId {
        let m = self.markets.get_mut(&market).expect("no such market");
        assert_eq!(order.price % m.tick_size, 0, "price off tick");
        let id = m.next_order_id;
        m.next_order_id += 1;
        m.orders.insert(id, order);
        id
    }

    /// The reference outlives the `at_mut` view temporary.
    pub fn fill(&mut self, market: MarketId, order: OrderId, qty: U128) -> U128 {
        let o = self
            .markets
            .at_mut(&market)
            .expect("no such market")
            .orders
            .get_mut(&order)
            .expect("no such order");
        o.qty -= qty.0;
        o.qty.into()
    }

    /// D9: removes a market in bounded steps, at most `n` orders per call. `true` while orders
    /// remain; meanwhile the market refuses new orders.
    pub fn close_market(&mut self, market: MarketId, n: u32) -> bool {
        self.markets.remove_some(&market, n)
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
        (6, 8, 0),
        "root, account entry + flag key (D9: a miss consults it) + header, usdt entry + header \\
         reads; account entry, index, slot, header and usdt entry, index, slot, header writes \\
         (`accounts` is a `Map` since D3)"
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
    let (p1, r) = view(|c| c.balances(alice(), None).unwrap());
    assert_eq!(p1.items.len(), 50);
    assert_eq!(p1.items[0], ("usdt".to_string(), 10));
    assert_eq!(r.reads(), 1 + 1 + 2 * 50, "root + header + 2 per item");
    let (p2, r) = view(|c| c.balances(alice(), p1.next.clone()).unwrap());
    assert_eq!(r.reads(), 1 + 2 + 2 * 50, "root + cursor index + header + 2 per item");
    let (p3, _) = view(|c| c.balances(alice(), p2.next.clone()).unwrap());
    assert_eq!(p3.items.len(), 21);
    assert_eq!(p3.next, None);
    // Page serializes to JSON as the spec says
    let json = near_sdk::serde_json::to_string(&p3).unwrap();
    assert!(json.starts_with("{\"items\":[[\"t"));
}

#[test]
fn markets_place_and_cancel() {
    init("owner.near");
    let (_, r) = call("owner.near", |c| c.open_market(7, U128(1)));
    assert_eq!(r.rwd(), (3, 4, 0), "root, index, header reads; market entry, index, slot, header");
    for i in 0..3 {
        let (id, _) = call("alice.near", |c| c.place(7, Order { price: 1, qty: i }));
        assert_eq!(id, i as u64);
    }
    let (_, r) = call("alice.near", |c| c.cancel(7, 0));
    // root; orders entry, index, header, last slot reads; moved slot + index + header writes;
    // 3 removes. The market is 1 existence check (`at_mut`), not read: a record being cleared
    // (D9) lives at its flag key, so a present record key means "not being cleared".
    assert_eq!(r.rwd(), (5, 3, 3));
    assert_eq!(r.has_keys(), 1);
    let (_, r) = call("alice.near", |c| c.cancel(7, 99));
    assert_eq!(r.rwd(), (2, 0, 0), "cancelling an unknown order: root, entry read");
    assert_eq!(r.has_keys(), 1, "the market's existence");
    let (n, _) = view(|c| c.markets.at(&7).orders.len());
    assert_eq!(n, 2);
}

#[test]
#[should_panic(expected = "no such market")]
fn cancel_in_an_absent_market_panics() {
    init("owner.near");
    call("alice.near", |c| c.cancel(9, 0));
}

/// The `at_mut(..).orders.get_mut(..)` reference outlives the view temporary, and a record's
/// scalars and its collection change together through `get_mut`.
#[test]
fn market_record_scalars_and_orders() {
    init("owner.near");
    call("owner.near", |c| c.open_market(7, U128(5)));
    call("alice.near", |c| {
        c.place(7, Order { price: 5, qty: 10 });
        c.place(7, Order { price: 10, qty: 20 });
    });
    let (left, r) = call("alice.near", |c| c.fill(7, 1, U128(4)));
    assert_eq!(left.0, 16);
    assert_eq!(r.rwd(), (2, 1, 0), "root, order; order write (the market is not read)");
    assert_eq!(r.has_keys(), 1, "the market's existence");
    let (m, _) = view(|c| {
        let m = c.markets.get(&7).unwrap();
        (m.tick_size, m.next_order_id)
    });
    assert_eq!(m, (5, 2));
}

#[test]
#[should_panic(expected = "price off tick")]
fn place_off_tick_panics() {
    init("owner.near");
    call("owner.near", |c| c.open_market(7, U128(5)));
    call("alice.near", |c| c.place(7, Order { price: 3, qty: 1 }));
}

#[test]
#[should_panic(expected = "record exists")]
fn opening_a_market_twice_panics() {
    init("owner.near");
    call("owner.near", |c| c.open_market(7, U128(5)));
    call("owner.near", |c| c.open_market(7, U128(5)));
}

// ---------------------------------------------------------------------------------------
// D9: clearing a record across calls
// ---------------------------------------------------------------------------------------

fn market_key(m: MarketId) -> Vec<u8> {
    near_sdk::state::Path::root(0xF0).field(4).kind(0).entry(&m).as_bytes().to_vec()
}

/// D9: the market's clearing flag key, its record key plus one `00` byte.
fn market_flag_key(m: MarketId) -> Vec<u8> {
    near_sdk::state::Path::root(0xF0).field(4).kind(0).entry(&m).flag().as_bytes().to_vec()
}

fn keys_under(prefix: &[u8]) -> usize {
    testing::store().keys().filter(|k| k.starts_with(prefix)).count()
}

fn panics(f: impl FnOnce()) -> String {
    let e = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)).expect_err("should panic");
    e.downcast_ref::<String>().cloned().unwrap_or_default()
}

fn market_with_orders(n: u64) {
    init("owner.near");
    call("owner.near", |c| c.open_market(7, U128(1)));
    call("alice.near", |c| {
        for i in 0..n {
            c.place(7, Order { price: 1, qty: i as u128 });
        }
    });
}

/// D9: a two-call clear with an interleaved insert. Call 1 clears 2 of 4 orders and moves the
/// market record to its clearing flag key (record key + `00`). Between the two clear calls,
/// every write path into the market panics (`get_mut` via `place`, `at_mut` via `cancel`,
/// `entry`), while `get` still reads it. Call 2 finishes: the market and all its keys are gone,
/// and a new market at the same id starts clean.
#[test]
fn clear_across_two_calls_refuses_an_interleaved_insert() {
    market_with_orders(4);
    let key = market_key(7);
    let flag = market_flag_key(7);
    assert_eq!(flag.len(), key.len() + 1);
    let before = testing::store()[&key].clone();
    assert!(!testing::store().contains_key(&flag));

    let (more, r) = call("owner.near", |c| c.close_market(7, 2));
    assert!(more, "2 orders remain");
    assert!(!testing::store().contains_key(&key), "the record key is absent while clearing");
    let flagged = testing::store()[&flag].clone();
    assert_eq!(flagged, before, "the record's bytes, unchanged, now at the flag key");
    assert!(r.keys_written().contains(&flag));
    assert!(r.ops.iter().any(|o| o.kind == testing::OpKind::Remove && o.key == key));
    assert_eq!(view(|c| c.markets.at(&7).orders.len()).0, 2);

    // interleaved writes: all refused
    let msg = panics(|| {
        call("alice.near", |c| c.place(7, Order { price: 1, qty: 9 }));
    });
    assert!(msg.contains("is being cleared by `remove_some`"), "{msg}");
    let msg = panics(|| {
        call("alice.near", |c| c.cancel(7, 0));
    });
    assert!(msg.contains("is being cleared"), "{msg}");
    let msg = panics(|| {
        call("alice.near", |c| {
            c.markets.entry(7).or_default().tick_size = 2;
        });
    });
    assert!(msg.contains("is being cleared"), "{msg}");
    // reads still work, and nothing was written by the refused calls
    assert_eq!(view(|c| c.markets.get(&7).map(|m| m.tick_size)).0, Some(1));
    assert_eq!(testing::store()[&flag], flagged);
    assert!(!testing::store().contains_key(&key));

    let (more, _) = call("owner.near", |c| c.close_market(7, 2));
    assert!(!more, "finished");
    assert_eq!(keys_under(&key), 0, "market record, flag key and every order key removed");
    assert_eq!(view(|c| c.markets.len()).0, 0);

    call("owner.near", |c| c.open_market(7, U128(3)));
    assert!(testing::store().contains_key(&key) && !testing::store().contains_key(&flag));
    call("alice.near", |c| c.place(7, Order { price: 3, qty: 1 }));
}

/// D9: a clear that fits in one call writes no flag; an absent record is `false` at once.
#[test]
fn clear_that_fits_in_one_call_writes_no_flag() {
    market_with_orders(3);
    let key = market_key(7);
    let flag = market_flag_key(7);
    let (more, r) = call("owner.near", |c| c.close_market(7, 3));
    assert!(!more);
    assert_eq!(keys_under(&key), 0);
    assert!(r.ops.iter().all(|o| o.key != flag), "the flag key is never touched");
    assert!(r.ops.iter().all(|o| o.kind != testing::OpKind::Write || o.key != key));
    assert!(!call("owner.near", |c| c.close_market(7, 3)).0, "absent: done");
}

/// D9 in one call: after `remove_some` returns `true`, the same call cannot write below the
/// record either.
#[test]
#[should_panic(expected = "is being cleared")]
fn writes_below_a_clearing_record_panic_in_the_same_call() {
    market_with_orders(4);
    call("owner.near", |c| {
        assert!(c.close_market(7, 1));
        c.place(7, Order { price: 1, qty: 1 });
    });
}

/// D7: `set_balance` writes below the account without decoding it, and an absent account is
/// the `expect` message, not a created orphan.
#[test]
fn set_balance_writes_below_without_decoding_the_record() {
    init("owner.near");
    call("alice.near", |c| c.deposit("usdt".into(), U128(5)));
    let (_, r) = call("owner.near", |c| c.set_balance(alice(), "usdt".into(), U128(9)));
    assert_eq!(r.has_keys(), 1, "the account's existence; the D9 flag key is not checked");
    assert_eq!(r.rwd(), (2, 1, 0), "root, balances index; entry write (the account is not read)");
    assert_eq!(view(|c| c.balance_of(alice(), "usdt".into())).0.0, 9);
}

#[test]
#[should_panic(expected = "no such account")]
fn set_balance_on_an_absent_account_panics() {
    init("owner.near");
    call("owner.near", |c| c.set_balance(alice(), "usdt".into(), U128(9)));
}

/// D8: the `balances` view returns `Err(CursorGone)` for a removed cursor.
#[test]
fn balances_with_a_removed_cursor_is_cursor_gone() {
    init("owner.near");
    call("alice.near", |c| c.deposit("a".into(), U128(1)));
    call("alice.near", |c| c.deposit("b".into(), U128(1)));
    call("alice.near", |c| {
        c.accounts.get_mut(&alice()).unwrap().balances.remove(&"a".to_string());
    });
    let (p, _) = view(|c| c.balances(alice(), Some("a".into())));
    assert_eq!(p, Err(CursorGone));
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
    c.accounts.insert_new(alice(), Account::default());
}

/// Item 12: `.bind()` in `#[init]` lets the body seed collections; the wrapper's write
/// flushes them before the root.
#[test]
fn init_with_bind_seeds_collections() {
    testing::use_mock();
    let c = Exchange::new_seeded("owner.near".parse().unwrap());
    c.state_write();
    let r = testing::receipt();
    // or_default reads the entry, its D9 flag key (the entry missed) and (absent) the header:
    // 3 reads; entry, index, slot, header and root writes (`accounts` is a `Map` since D3)
    assert_eq!(r.rwd(), (3, 5, 0));
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
    let mut v1: LookupMap<u32, SlimV1> = LookupMap::__bound_at(root.clone());
    v1.insert(1, SlimV1 { nonce: 3 });
    testing::flush();
    let v2: LookupMap<u32, SlimV2> = LookupMap::__bound_at(root);
    let _ = v2.get(&1).map(|s| (&s.owner, s.nonce));
}

// ---------------------------------------------------------------------------------------
// D2: `#[key(n, default = expr)]`, serde-style
// ---------------------------------------------------------------------------------------

#[near(storage)]
pub struct Defaults {
    #[key(1)]
    nonce: u64,
    #[key(2, default = 30)]
    fee_bps: u16,
    // No `Default` for AccountId, and no `required` or `Option` needed.
    #[key(3, default = "treasury.near".parse().unwrap())]
    treasury: AccountId,
    #[key(4, default = vec![1, 2])]
    tiers: Vec<u8>,
}

#[near(storage)]
pub struct Required {
    #[key(1, required)]
    owner: AccountId,
    #[key(2)]
    nonce: u64,
}

/// D2 derive: a missing tag evaluates the expression, in `decode` and in the generated
/// `Default`; a present tag decodes as usual.
#[test]
fn key_default_expr_fills_missing_tags() {
    use near_sdk::state::Path;
    let d = Defaults::default();
    assert_eq!((d.nonce, d.fee_bps, d.treasury.as_str(), d.tiers.clone()), (0, 30, "treasury.near", vec![1, 2]));

    testing::use_mock();
    let root = Path::root(0xF0).field(9);
    let mut m: LookupMap<u32, Defaults> = LookupMap::__bound_at(root);
    m.insert(1, Defaults { fee_bps: 5, ..Defaults::default() });
    testing::flush();
    testing::begin_call();
    let d = m.get(&1).unwrap();
    assert_eq!((d.fee_bps, d.treasury.as_str()), (5, "treasury.near"), "present tag wins");
}

/// D2 derive: `required` keeps working: present decodes, absent is an error naming the tag.
#[test]
fn key_required_still_decodes_and_fails_when_missing() {
    use near_sdk::state::Path;
    testing::use_mock();
    let root = Path::root(0xF0).field(9);
    let mut m: LookupMap<u32, Required> = LookupMap::__bound_at(root.clone());
    m.insert(1, Required { owner: alice(), nonce: 2 });
    testing::flush();
    testing::begin_call();
    assert_eq!(m.get(&1).unwrap().owner, alice());
    // A stored record with only tag 2.
    let mut w = near_sdk::state::record::RecordWriter::new();
    w.push(2, near_sdk::borsh::to_vec(&3u64).unwrap());
    testing::seed(root.kind(0).entry(&2u32).as_bytes(), &w.finish(None, |_| true));
    let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _ = m.get(&2).map(|r| r.nonce);
    }));
    assert!(r.is_err());
}

#[near(storage)]
pub struct SlimV2Default {
    #[key(1)]
    nonce: u64,
    #[key(2, default = "owner.near".parse().unwrap())]
    owner: AccountId, // added in v2 the right way
}

fn write_slim_v1(root: &near_sdk::state::Path) {
    let mut v1: LookupMap<u32, SlimV1> = LookupMap::__bound_at(root.clone());
    v1.insert(1, SlimV1 { nonce: 3 });
    testing::flush();
    testing::begin_call();
}

/// D2 cross-schema: a field added with `default = ..` decodes a record written before it
/// existed.
#[test]
fn field_added_with_default_decodes_an_old_record() {
    use near_sdk::state::Path;
    testing::use_mock();
    let root = Path::root(0xF0).field(9);
    write_slim_v1(&root);
    let v2: LookupMap<u32, SlimV2Default> = LookupMap::__bound_at(root);
    let s = v2.get(&1).unwrap();
    assert_eq!((s.nonce, s.owner.as_str()), (3, "owner.near"));
}

/// D2 cross-schema: the same field added as `required` fails every old record, and the
/// message names the tag (and the fix).
#[test]
#[should_panic(expected = "required field tag 2 is absent from the stored record")]
fn field_added_as_required_fails_an_old_record_naming_the_tag() {
    use near_sdk::state::Path;
    testing::use_mock();
    let root = Path::root(0xF0).field(9);
    write_slim_v1(&root);
    let v2: LookupMap<u32, SlimV2> = LookupMap::__bound_at(root);
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
    users: LookupMap<u32, User>,
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
    let mut m: LookupMap<u32, Slim> = LookupMap::__bound_at(root.clone());
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

/// D4: unknown tags are PRESERVED through every read-modify-write path (`get_mut` and
/// `entry(..).or_default()`), and a tag listed in `reserved(n)` is dropped on rewrite.
#[test]
fn unknown_tags_survive_get_mut_and_entry_reserved_tags_are_dropped() {
    use near_sdk::state::{Path, record};
    testing::use_mock();
    let root = Path::root(0xF0).field(9);
    let mut m: Map<u32, Slim> = Map::__bound_at(root.clone());
    m.insert(1, Slim { nonce: 1 });
    m.insert(2, Slim { nonce: 1 });
    testing::flush();
    let mut w = record::RecordWriter::new();
    w.push(1, near_sdk::borsh::to_vec(&1u64).unwrap());
    w.push(3, vec![0x33]); // reserved: a deliberately retired field
    w.push(4, vec![0x44]); // unknown: a field from a newer version
    let bytes = w.finish(None, |_| true);
    let k1 = root.kind(0).entry(&1u32);
    let k2 = root.kind(0).entry(&2u32);
    testing::seed(k1.as_bytes(), &bytes);
    testing::seed(k2.as_bytes(), &bytes);
    testing::begin_call();
    m.get_mut(&1).unwrap().nonce = 2;
    m.entry(2).or_default().nonce = 2;
    testing::flush();
    for k in [k1, k2] {
        let tags: Vec<u32> = record::parse(&testing::store()[k.as_bytes()])
            .unwrap()
            .into_iter()
            .map(|(t, _)| t)
            .collect();
        assert_eq!(tags, vec![1, 4], "unknown 4 kept, reserved 3 dropped");
    }
}

#[near(storage)]
pub struct VaultV1 {
    #[key(1)]
    nonce: u64,
    #[key(2)]
    locked: u128,
}

/// v2 deletes `locked` but forgets `reserved(2)`.
#[near(storage)]
pub struct VaultV2 {
    #[key(1)]
    nonce: u64,
}

/// v3 reuses tag 2 for a different meaning.
#[near(storage)]
pub struct VaultV3 {
    #[key(1)]
    nonce: u64,
    #[key(2)]
    claimable: u128,
}

/// D4 HAZARD (audit finding), pinned so it reads as a hazard, not a feature: a removed field's
/// bytes persist through every rewrite until its tag is listed in `reserved(n)`, so reusing the
/// tag later resurrects them. Here stale `locked` funds reappear as `claimable`. The fix is a
/// build error for tag reuse against the deployed ABI (not built in the prototype); until then,
/// always `reserved(n)` a removed tag.
#[test]
fn hazard_reusing_a_removed_tag_resurrects_its_stale_bytes() {
    use near_sdk::state::Path;
    testing::use_mock();
    let root = Path::root(0xF0).field(9);
    let mut v1: Map<u32, VaultV1> = Map::__bound_at(root.clone());
    v1.insert(1, VaultV1 { nonce: 1, locked: 1_000 });
    testing::flush();
    testing::begin_call();
    let mut v2: Map<u32, VaultV2> = Map::__bound_at(root.clone());
    v2.get_mut(&1).unwrap().nonce = 2; // rewrite under v2: tag 2 is unknown, so preserved
    testing::flush();
    testing::begin_call();
    let v3: Map<u32, VaultV3> = Map::__bound_at(root);
    let v = v3.get(&1).unwrap();
    assert_eq!((v.nonce, v.claimable), (2, 1_000), "HAZARD: stale `locked` read as `claimable`");
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
