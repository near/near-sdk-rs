//! The spec's Exchange, compiled for wasm32 so the generated wrappers are type-checked.
use near_sdk::json_types::U128;
use near_sdk::state::{GroupedMap, IterMap, Lazy, Map, Page};
use near_sdk::{AccountId, PanicOnDefault, env, near};

pub type TokenId = String;
pub type MarketId = u32;
pub type OrderId = u64;

#[near(serializers = [borsh, json])]
#[derive(Clone)]
pub struct Order {
    pub price: u128,
    pub qty: u128,
}
near_sdk::state::removable_leaf!(Order);

#[near(serializers = [borsh])]
#[derive(Default)]
pub struct Metadata {
    pub name: String,
}

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
    metadata: Lazy<Metadata>,
}

#[near(storage)]
#[derive(Default)]
pub struct Account {
    #[key(1)]
    nonce: u64,
    #[key(2)]
    balances: IterMap<TokenId, u128>,
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

    pub fn deposit(&mut self, token: TokenId, amount: U128) {
        let acct = self.accounts.entry(env::predecessor_account_id()).or_default();
        *acct.balances.entry(token).or_default() += amount.0;
    }

    pub fn balance_of(&self, id: AccountId, token: TokenId) -> U128 {
        self.accounts.at(&id).balances.get(&token).copied().unwrap_or(0).into()
    }

    pub fn balances(&self, id: AccountId, after: Option<TokenId>) -> Page<TokenId, u128> {
        self.accounts.at(&id).balances.page(after.as_ref(), 50)
    }

    pub fn cancel(&mut self, market: MarketId, order: OrderId) {
        self.orders.group_mut(&market).remove(&order);
    }

    pub fn fee(&self) -> u16 {
        let _ = &self.metadata;
        self.fee_bps
    }
}
