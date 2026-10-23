//! "Rekey when worth it": the accounts map wrapped in `Migrating`.
//!
//! `FungibleToken` is a near-contract-standards struct whose `accounts` field is a concrete
//! `collections::LookupMap`. One field of it cannot be wrapped, so this flattens it into root
//! fields and re-implements the handful of methods the example exposes. That fork is the cost
//! of rekeying for any contract built on the standards.

use near_contract_standards::fungible_token::FungibleToken;
use near_contract_standards::fungible_token::metadata::FungibleTokenMetadata;
use near_sdk::collections::{LazyOption, LookupMap as V1LookupMap};
use near_sdk::json_types::U128;
use near_sdk::state::{LookupMap, Map, Migrating};
use near_sdk::{AccountId, NearToken, assert_one_yocto, env, near, require};

pub type Accounts = Migrating<V1LookupMap<AccountId, u128>, LookupMap<AccountId, u128>>;

#[near(contract_state(layout = v2), reserved(1))]
pub struct Contract {
    // tag 1 was `token: FungibleToken` in v2; retired.
    #[key(2, legacy)]
    pub metadata: LazyOption<FungibleTokenMetadata>,
    #[key(3)]
    pub transfers_sent: Map<AccountId, u64>,
    /// v1 prefix `[0]` inline, v2 entries under `0xF0 ++ 4`.
    #[key(4, required)]
    pub accounts: Accounts,
    #[key(5)]
    pub total_supply: u128,
    /// Measured by v1 for v1 key lengths. Not re-measured (see the notes).
    #[key(6)]
    pub account_storage_usage: u64,
}

#[near]
impl Contract {
    /// From the v2 root. `ignore_state` is required: the v2 root exists, so a plain `#[init]`
    /// refuses. Guarded instead by tag 1 being present (it is dropped by this call's flush,
    /// since it is reserved).
    #[private]
    #[init(ignore_state)]
    pub fn migrate() -> Self {
        let fields = near_sdk::state::raw_root::<Self>()
            .unwrap_or_else(|| env::panic_str("no v2 root to migrate from"));
        let tag = |t: u32| fields.iter().find(|(tag, _)| *tag == t).map(|(_, b)| b.as_slice());
        let token: FungibleToken =
            near_sdk::borsh::from_slice(tag(1).unwrap_or_else(|| env::panic_str("already migrated")))
                .unwrap_or_else(|_| env::panic_str("tag 1 is not a FungibleToken"));
        let metadata: LazyOption<FungibleTokenMetadata> =
            near_sdk::borsh::from_slice(tag(2).expect("metadata")).expect("metadata");
        Self {
            metadata,
            transfers_sent: Default::default(),
            accounts: Migrating::new(token.accounts),
            total_supply: token.total_supply,
            account_storage_usage: token.account_storage_usage,
        }
    }

    #[payable]
    pub fn ft_transfer(&mut self, receiver_id: AccountId, amount: U128, memo: Option<String>) {
        assert_one_yocto();
        let _ = memo;
        let sender_id = env::predecessor_account_id();
        require!(sender_id != receiver_id, "Sender and receiver should be different");
        require!(amount.0 > 0, "The amount should be a positive number");
        let from = self.balance(&sender_id);
        let to = self.balance(&receiver_id);
        let from = from.checked_sub(amount.0).unwrap_or_else(|| {
            env::panic_str("The account doesn't have enough balance")
        });
        self.accounts.insert(sender_id.clone(), from);
        self.accounts.insert(receiver_id, to.checked_add(amount.0).expect("Balance overflow"));
        *self.transfers_sent.entry(sender_id).or_default() += 1;
    }

    pub fn ft_balance_of(&self, account_id: AccountId) -> U128 {
        U128(self.accounts.get(&account_id).unwrap_or(0))
    }

    pub fn ft_total_supply(&self) -> U128 {
        U128(self.total_supply)
    }

    /// Registration only (the example's `storage_deposit` without the refund paths). New
    /// accounts land on the v2 side, whose keys are longer than the v1 bond was measured for.
    #[payable]
    pub fn storage_deposit(&mut self, account_id: Option<AccountId>) {
        let account_id = account_id.unwrap_or_else(env::predecessor_account_id);
        let bond = env::storage_byte_cost().saturating_mul(self.account_storage_usage.into());
        require!(env::attached_deposit() >= bond, "deposit below the storage bond");
        require!(!self.accounts.contains_key(&account_id), "already registered");
        self.accounts.insert(account_id, 0);
    }

    /// Moves up to `n` accounts. Does not compile for this contract: the v1 side is a
    /// `LookupMap`, which cannot enumerate its keys. Kept as a comment to show the probe:
    ///
    /// ```text
    /// pub fn migrate_some(&mut self, n: u32) -> u32 { self.accounts.migrate_batch(n) }
    /// ```
    pub fn storage_balance_bounds_min(&self) -> NearToken {
        env::storage_byte_cost().saturating_mul(self.account_storage_usage.into())
    }

    pub fn transfers_sent_by(&self, account_id: AccountId) -> u64 {
        self.transfers_sent.get(&account_id).copied().unwrap_or(0)
    }
}

impl Contract {
    fn balance(&self, a: &AccountId) -> u128 {
        self.accounts
            .get(a)
            .unwrap_or_else(|| env::panic_str(&format!("The account {a} is not registered")))
    }
}
