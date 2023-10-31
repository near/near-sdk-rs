use crate::fungible_token::core::FungibleTokenCore;
use crate::fungible_token::events::{FtBurn, FtTransfer};
use crate::fungible_token::receiver::ext_ft_receiver;
use crate::fungible_token::resolver::{ext_ft_resolver, FungibleTokenResolver};
use near_sdk::borsh::{BorshDeserialize, BorshSerialize};
use near_sdk::collections::LookupMap;
use near_sdk::json_types::U128;
use near_sdk::{
    assert_one_yocto, env, log, require, AccountId, Gas, IntoStorageKey, NearToken, PromiseOrValue,
    PromiseResult, StorageUsage,
};

const GAS_FOR_RESOLVE_TRANSFER: Gas = Gas::from_tgas(5);
const GAS_FOR_FT_TRANSFER_CALL: Gas = Gas::from_tgas(30);

const ERR_TOTAL_SUPPLY_OVERFLOW: &str = "Total supply overflow";

/// Implementation of a FungibleToken standard.
/// Allows to include NEP-141 compatible token to any contract.
/// There are next traits that any contract may implement:
///     - FungibleTokenCore -- interface with ft_transfer methods. FungibleToken provides methods for it.
///     - FungibleTokenMetaData -- return metadata for the token in NEP-148, up to contract to implement.
///     - StorageManager -- interface for NEP-145 for allocating storage per account. FungibleToken provides methods for it.
///     - AccountRegistrar -- interface for an account to register and unregister
///
/// For example usage, see examples/fungible-token/src/lib.rs.
#[derive(BorshDeserialize, BorshSerialize)]
#[borsh(crate = "near_sdk::borsh")]
pub struct FungibleToken {
    /// AccountID -> Account balance.
    pub accounts: LookupMap<AccountId, NearToken>,

    /// Total supply of the all token.
    pub total_supply: NearToken,

    /// The storage size in bytes for one account.
    pub account_storage_usage: StorageUsage,
}

impl FungibleToken {
    pub fn new<S>(prefix: S) -> Self
    where
        S: IntoStorageKey,
    {
        let mut this = Self {
            accounts: LookupMap::new(prefix),
            total_supply: NearToken::from_near(0),
            account_storage_usage: 0,
        };
        this.measure_account_storage_usage();
        this
    }

    fn measure_account_storage_usage(&mut self) {
        let initial_storage_usage = env::storage_usage();
        let tmp_account_id = AccountId::new_unchecked("a".repeat(64));
        self.accounts.insert(&tmp_account_id, &NearToken::from_near(0));
        self.account_storage_usage = env::storage_usage() - initial_storage_usage;
        self.accounts.remove(&tmp_account_id);
    }

    pub fn internal_unwrap_balance_of(&self, account_id: &AccountId) -> NearToken {
        match self.accounts.get(account_id) {
            Some(balance) => balance,
            None => {
                env::panic_str(format!("The account {} is not registered", &account_id).as_str())
            }
        }
    }

    pub fn internal_deposit(&mut self, account_id: &AccountId, amount: NearToken) {
        let balance = self.internal_unwrap_balance_of(account_id);
        if let Some(new_balance) = balance.checked_add(amount) {
            self.accounts.insert(account_id, &new_balance);
            self.total_supply = self
                .total_supply
                .checked_add(amount)
                .unwrap_or_else(|| env::panic_str(ERR_TOTAL_SUPPLY_OVERFLOW));
        } else {
            env::panic_str("Balance overflow");
        }
    }

    pub fn internal_withdraw(&mut self, account_id: &AccountId, amount: NearToken) {
        let balance = self.internal_unwrap_balance_of(account_id);
        if let Some(new_balance) = balance.checked_sub(amount) {
            self.accounts.insert(account_id, &new_balance);
            self.total_supply = self
                .total_supply
                .checked_sub(amount)
                .unwrap_or_else(|| env::panic_str(ERR_TOTAL_SUPPLY_OVERFLOW));
        } else {
            env::panic_str("The account doesn't have enough balance");
        }
    }

    pub fn internal_transfer(
        &mut self,
        sender_id: &AccountId,
        receiver_id: &AccountId,
        amount: NearToken,
        memo: Option<String>,
    ) {
        require!(sender_id != receiver_id, "Sender and receiver should be different");
        require!(amount > NearToken::from_near(0), "The amount should be a positive number");
        self.internal_withdraw(sender_id, amount);
        self.internal_deposit(receiver_id, amount);
        FtTransfer {
            old_owner_id: sender_id,
            new_owner_id: receiver_id,
            amount: &amount,
            memo: memo.as_deref(),
        }
        .emit();
    }

    pub fn internal_register_account(&mut self, account_id: &AccountId) {
        if self.accounts.insert(account_id, &NearToken::from_near(0)).is_some() {
            env::panic_str("The account is already registered");
        }
    }
}

impl FungibleTokenCore for FungibleToken {
    fn ft_transfer(&mut self, receiver_id: AccountId, amount: NearToken, memo: Option<String>) {
        assert_one_yocto();
        let sender_id = env::predecessor_account_id();
        self.internal_transfer(&sender_id, &receiver_id, amount, memo);
    }

    fn ft_transfer_call(
        &mut self,
        receiver_id: AccountId,
        amount: NearToken,
        memo: Option<String>,
        msg: String,
    ) -> PromiseOrValue<NearToken> {
        assert_one_yocto();
        require!(env::prepaid_gas() > GAS_FOR_FT_TRANSFER_CALL, "More gas is required");
        let sender_id = env::predecessor_account_id();
        self.internal_transfer(&sender_id, &receiver_id, amount, memo);
        let receiver_gas = env::prepaid_gas()
            .checked_sub(GAS_FOR_FT_TRANSFER_CALL)
            .unwrap_or_else(|| env::panic_str("Prepaid gas overflow"));
        // Initiating receiver's call and the callback
        ext_ft_receiver::ext(receiver_id.clone())
            .with_static_gas(receiver_gas)
            .ft_on_transfer(sender_id.clone(), amount, msg)
            .then(
                ext_ft_resolver::ext(env::current_account_id())
                    .with_static_gas(GAS_FOR_RESOLVE_TRANSFER)
                    .ft_resolve_transfer(sender_id, receiver_id, amount),
            )
            .into()
    }

    fn ft_total_supply(&self) -> NearToken {
        self.total_supply
    }

    fn ft_balance_of(&self, account_id: AccountId) -> NearToken {
        self.accounts.get(&account_id).unwrap_or(NearToken::from_near(0))
    }
}

impl FungibleToken {
    /// Internal method that returns the amount of burned tokens in a corner case when the sender
    /// has deleted (unregistered) their account while the `ft_transfer_call` was still in flight.
    /// Returns (Used token amount, Burned token amount)
    pub fn internal_ft_resolve_transfer(
        &mut self,
        sender_id: &AccountId,
        receiver_id: AccountId,
        amount: NearToken,
    ) -> (u128, u128) {
        let amount: NearToken = amount.into();

        // Get the unused amount from the `ft_on_transfer` call result.
        let unused_amount = match env::promise_result(0) {
            PromiseResult::Successful(value) => {
                if let Ok(unused_amount) = near_sdk::serde_json::from_slice::<NearToken>(&value) {
                    std::cmp::min(amount, unused_amount)
                } else {
                    amount
                }
            }
            PromiseResult::Failed => amount,
        };

        if unused_amount.as_yoctonear() > 0 {
            let receiver_balance =
                self.accounts.get(&receiver_id).unwrap_or(NearToken::from_near(0));
            if receiver_balance.as_yoctonear() > 0 {
                let refund_amount = std::cmp::min(receiver_balance, unused_amount);
                if let Some(new_receiver_balance) = receiver_balance.checked_sub(refund_amount) {
                    self.accounts.insert(&receiver_id, &new_receiver_balance);
                } else {
                    env::panic_str("The receiver account doesn't have enough balance");
                }

                if let Some(sender_balance) = self.accounts.get(sender_id) {
                    if let Some(new_sender_balance) = sender_balance.checked_add(refund_amount) {
                        self.accounts.insert(sender_id, &new_sender_balance);
                    } else {
                        env::panic_str("Sender balance overflow");
                    }

                    FtTransfer {
                        old_owner_id: &receiver_id,
                        new_owner_id: sender_id,
                        amount: &refund_amount,
                        memo: Some("refund"),
                    }
                    .emit();
                    let used_amount = amount
                        .checked_sub(refund_amount)
                        .unwrap_or_else(|| env::panic_str(ERR_TOTAL_SUPPLY_OVERFLOW));
                    return (used_amount.as_yoctonear(), 0);
                } else {
                    // Sender's account was deleted, so we need to burn tokens.
                    self.total_supply = self
                        .total_supply
                        .checked_sub(refund_amount)
                        .unwrap_or_else(|| env::panic_str(ERR_TOTAL_SUPPLY_OVERFLOW));
                    log!("The account of the sender was deleted");
                    FtBurn { owner_id: &receiver_id, amount: &refund_amount, memo: Some("refund") }
                        .emit();
                    return (amount.as_yoctonear(), refund_amount.as_yoctonear());
                }
            }
        }
        (amount.as_yoctonear(), 0)
    }
}

impl FungibleTokenResolver for FungibleToken {
    fn ft_resolve_transfer(
        &mut self,
        sender_id: AccountId,
        receiver_id: AccountId,
        amount: NearToken,
    ) -> NearToken {
        NearToken::from_millinear(
            self.internal_ft_resolve_transfer(&sender_id, receiver_id, amount).0,
        )
    }
}
