//! Revision 1 success check: the spec's Exchange, verbatim, with `required` on `owner` as the
//! only edit. Compiled natively and for wasm32 (so the real `#[near]` wrappers are checked).
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

// ---------------- spec text from here, verbatim except `required` ----------------

use near_sdk::state::{LookupMap, Map, GroupedMap, Lazy, Page};
use near_sdk::{env, near, AccountId};
use near_sdk::json_types::U128;

#[near(contract_state(layout = v2))]
pub struct Exchange {
    #[key(1, required)] owner: AccountId,
    #[key(2)] fee_bps: u16,
    #[key(3)] accounts: LookupMap<AccountId, Account>,
    #[key(4)] orders: GroupedMap<MarketId, OrderId, Order>,
    #[key(5)] metadata: Lazy<Metadata>,          // large, loaded only when touched
}

#[near(storage)]
pub struct Account {
    #[key(1)] nonce: u64,
    #[key(2)] balances: Map<TokenId, u128>,  // lives at 3/<account>/2/<token>
}

#[near]
impl Exchange {
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
}
