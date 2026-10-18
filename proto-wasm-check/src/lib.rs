//! Success check: the spec's Exchange, verbatim, compiled natively and for wasm32 (so the real
//! `#[near]` wrappers are checked).
//!
//! Revision 3: the text below is the revised spec's (spec-rev.md, "What the developer
//! writes"), which uses the renamed types (`Map` for both maps), already carries `required`
//! on `owner`, and adds `set_balance` through `at_mut`. Rounds 1-2 checked the original
//! spec's text with `required` as the only edit.
//!
//! The spec does not define `TokenId`, `MarketId`, `OrderId`, `Order` or `Metadata`; they are
//! declared here as plain types with no storage attributes and no opt-ins.

pub type TokenId = String;
pub type MarketId = u32;
pub type OrderId = u64;

#[near_sdk::near(serializers = [borsh, json])]
#[derive(Clone)]
pub struct Order {
    pub price: u128,
    pub qty: u128,
}

#[near_sdk::near(serializers = [borsh])]
#[derive(Default)]
pub struct Metadata {
    pub name: String,
}

// ---------------- revised spec text from here, verbatim ----------------
// API round (Oct 17): D3 replaces `orders: GroupedMap<..>` with `markets: Map<MarketId, Market>`,
// D7 makes `at_mut` return `Option`, D8 makes `page` return `Result<Page, CursorGone>`. The
// same text is `near-sdk/tests/state_v2_exchange.rs`'s contract.

use near_sdk::state::{CursorGone, Lazy, Map, Page};
use near_sdk::{env, near, AccountId};
use near_sdk::json_types::U128;

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
}
