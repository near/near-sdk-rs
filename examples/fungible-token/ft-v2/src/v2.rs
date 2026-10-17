//! "Move the root": the v1 fungible token with a v2 root.

use near_contract_standards::fungible_token::metadata::{
    FungibleTokenMetadata, FungibleTokenMetadataProvider,
};
use near_contract_standards::fungible_token::{
    FungibleToken, FungibleTokenCore, FungibleTokenResolver,
};
use near_contract_standards::storage_management::{
    StorageBalance, StorageBalanceBounds, StorageManagement,
};
use near_sdk::collections::LazyOption;
use near_sdk::json_types::U128;
use near_sdk::state::Map;
use near_sdk::{AccountId, NearToken, PromiseOrValue, env, log, near};

/// The deployed v1 root, `examples/fungible-token`'s `Contract`, as its Borsh layout.
#[near(serializers = [borsh])]
pub struct ContractV1 {
    pub token: FungibleToken,
    pub metadata: LazyOption<FungibleTokenMetadata>,
}

#[near(contract_state(layout = v2))]
pub struct Contract {
    /// v1, kept at the root: `collections::LookupMap` under prefix `[0]`, written through.
    #[key(1, required)]
    pub token: FungibleToken,
    /// v1, kept at the root: `collections::LazyOption` under prefix `[1]`.
    #[key(2, required)]
    pub metadata: LazyOption<FungibleTokenMetadata>,
    /// New in v2: how many `ft_transfer`s each account has sent since the upgrade.
    #[key(3)]
    pub transfers_sent: Map<AccountId, u64>,
}

#[near]
impl Contract {
    /// One-time upgrade from the v1 layout. A plain `#[init]` is enough: the wrapper's
    /// "already initialized" check looks at the v2 root key, which does not exist yet, and
    /// does exist afterwards, so `migrate` cannot run twice.
    #[private]
    #[init]
    pub fn migrate() -> Self {
        let old: ContractV1 = env::state_read().unwrap_or_else(|| env::panic_str("no v1 STATE"));
        // Invented (the spec is silent): drop the old blob. Keeping it would make a rollback
        // to the v1 wasm read a stale `total_supply`.
        env::storage_remove(b"STATE");
        Self { token: old.token, metadata: old.metadata, transfers_sent: Default::default() }
    }

    pub fn transfers_sent_by(&self, account_id: AccountId) -> u64 {
        self.transfers_sent.get(&account_id).copied().unwrap_or(0)
    }
}

#[near]
impl FungibleTokenCore for Contract {
    #[payable]
    fn ft_transfer(&mut self, receiver_id: AccountId, amount: U128, memo: Option<String>) {
        self.token.ft_transfer(receiver_id, amount, memo);
        *self.transfers_sent.entry(env::predecessor_account_id()).or_default() += 1;
    }

    #[payable]
    fn ft_transfer_call(
        &mut self,
        receiver_id: AccountId,
        amount: U128,
        memo: Option<String>,
        msg: String,
    ) -> PromiseOrValue<U128> {
        self.token.ft_transfer_call(receiver_id, amount, memo, msg)
    }

    fn ft_total_supply(&self) -> U128 {
        self.token.ft_total_supply()
    }

    fn ft_balance_of(&self, account_id: AccountId) -> U128 {
        self.token.ft_balance_of(account_id)
    }
}

#[near]
impl FungibleTokenResolver for Contract {
    #[private]
    fn ft_resolve_transfer(
        &mut self,
        sender_id: AccountId,
        receiver_id: AccountId,
        amount: U128,
    ) -> U128 {
        let (used_amount, burned_amount) =
            self.token.internal_ft_resolve_transfer(&sender_id, receiver_id, amount);
        if burned_amount > 0 {
            log!("Account @{} burned {}", sender_id, burned_amount);
        }
        used_amount.into()
    }
}

#[near]
impl StorageManagement for Contract {
    #[payable]
    fn storage_deposit(
        &mut self,
        account_id: Option<AccountId>,
        registration_only: Option<bool>,
    ) -> StorageBalance {
        self.token.storage_deposit(account_id, registration_only)
    }

    #[payable]
    fn storage_withdraw(&mut self, amount: Option<NearToken>) -> StorageBalance {
        self.token.storage_withdraw(amount)
    }

    #[payable]
    fn storage_unregister(&mut self, force: Option<bool>) -> bool {
        self.token.internal_storage_unregister(force).is_some()
    }

    fn storage_balance_bounds(&self) -> StorageBalanceBounds {
        self.token.storage_balance_bounds()
    }

    fn storage_balance_of(&self, account_id: AccountId) -> Option<StorageBalance> {
        self.token.storage_balance_of(account_id)
    }
}

#[near]
impl FungibleTokenMetadataProvider for Contract {
    fn ft_metadata(&self) -> FungibleTokenMetadata {
        self.metadata.get().unwrap()
    }
}
