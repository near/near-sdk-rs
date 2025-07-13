use std::borrow::Cow;

use near_sdk::{
    borsh, env, json_types::U128, near, require, serde_json, AccountId, AccountIdRef, NearToken,
    PromiseOrValue, PromiseResult, StateInit, StateInitArgs, StateInitFunctionCall,
};

use crate::sharded_fungible_token::{
    minter::sft_burner,
    receiver::sft_receiver,
    wallet::{
        InitArgs, ShardedFungibleTokenWallet, ShardedFungibleTokenWalletData,
        ShardedFungibleTokenWalletDataExt, TransferNotificaton,
    },
};

/// Reference implementation of Sharded Fungible Foken wallet-contract.
///
/// This implementation should be globally deployed only once and can
/// then be reused for different minter-contracts. Owners might reference
/// its globally deployed code to calculate [`StateInit`] and verify
/// authenticity of [`env::predecessor_account_id()`] via
/// [`.derived_account_id()`](StateInit::derived_account_id).
///
/// The implementation is highly inspired by [Jetton wallet](https://github.com/ton-blockchain/jetton-contract/blob/3d24b419f2ce49c09abf6b8703998187fe358ec9/contracts/jetton-wallet.fc)
/// contract reference implementation.  
/// See [wallet-contract documentation](ShardedFungibleTokenWallet).
#[near]
impl ShardedFungibleTokenWallet for ShardedFungibleTokenWalletData {
    /// Intialize contract's state.
    #[init]
    fn init(
        // we use borsh for deterministic serialization
        #[serializer(borsh)] init_args: InitArgs<'static>,
    ) -> Self {
        Self {
            owner_id: init_args.owner_id.into_owned(),
            minter_id: init_args.minter_id.into_owned(),
            balance: U128(0),
        }
    }

    /// View method to get all data at once
    fn sft_wallet_data(&self) -> &ShardedFungibleTokenWalletData {
        self
    }

    /// Transfer given `amount` of tokens to `receiver_id`.
    ///
    /// Requires at least [`ShardedFungibleTokenWalletData::MIN_BALANCE`]
    /// attached deposit to reserve for deploying receiver's wallet-contract
    /// if it doesn't exist. If it turned out to be already deployed, then
    /// reserved NEAR tokens are sent to `wallet_init_refund_to`.
    ///
    /// If `notify` is set, then `receiver_id::sft_on_transfer()` will be
    /// called. If `notify.state_init` is set, then `receiver_id` will be
    /// initialized if doesn't exist.
    ///
    /// Remaining attached deposit is forwarded to `receiver_id::sft_on_transfer()`.
    ///
    /// Returns `used_amount`.
    #[payable]
    fn sft_transfer(
        &mut self,
        receiver_id: AccountId,
        amount: U128,
        memo: String,
        notify: Option<TransferNotificaton>,
        wallet_init_refund_to: Option<AccountId>,
    ) -> PromiseOrValue<U128> {
        require!(env::predecessor_account_id() == self.owner_id, "not owner");

        self.balance.0 = self
            .balance
            .0
            .checked_sub(amount.0)
            .unwrap_or_else(|| env::panic_str("insufficient balance"));

        let forward_deposit = env::attached_deposit()
            .checked_sub(Self::MIN_BALANCE)
            .filter(|d| *d >= NearToken::from_yoctonear(1))
            .unwrap_or_else(|| env::panic_str("insufficient attached deposit"));
        let receiver_wallet_init = self.state_init_for(&receiver_id);
        let receiver_wallet_id = receiver_wallet_init.derived_account_id();

        // call receiver_wallet_id::sft_receive()
        Self::ext(receiver_wallet_id)
            // forward all attached deposit
            .with_attached_deposit(forward_deposit)
            // require minimum gas
            .with_static_gas({
                if notify.is_some() {
                    Self::SFT_RECEIVE_MIN_GAS.saturating_add(Self::SFT_RESOLVE_GAS)
                } else {
                    Self::SFT_RECEIVE_MIN_GAS
                }
            })
            // forward all remaining gas here
            .with_unused_gas_weight(1)
            // deploy if not exist
            .with_state_init(Some(StateInitArgs {
                state_init: receiver_wallet_init,
                amount: Self::MIN_BALANCE,
                refund_to: wallet_init_refund_to.unwrap_or_else(env::predecessor_account_id),
            }))
            .sft_receive(self.owner_id.clone(), amount, memo, notify)
            .then(
                Self::ext(env::current_account_id())
                    .with_static_gas(Self::SFT_RESOLVE_GAS)
                    // do not distribute remaining gas here
                    .with_unused_gas_weight(0)
                    .sft_resolve(amount, true),
            )
            .into()
    }

    /// Receives tokens from minter-contract or wallet-contracts initialized
    /// for the same minter-contract.
    ///
    /// If `notify` is set, then `receiver_id::sft_on_transfer()` will be
    /// called. If `notify.state_init` is set, then `receiver_id` will be
    /// initialized if doesn't exist.
    ///
    /// Remaining attached deposit is forwarded to `receiver_id::sft_on_transfer()`.
    ///
    /// Returns `used_amount`.
    #[payable]
    fn sft_receive(
        &mut self,
        sender_id: AccountId,
        amount: U128,
        // memo will be stored in receipt's FunctionCall args anyway
        #[allow(unused_variables)] memo: String,
        notify: Option<TransferNotificaton>,
    ) -> PromiseOrValue<U128> {
        assert_at_least_one_yocto_near();
        let sender_wallet_id = env::predecessor_account_id();

        // verify sender is a valid wallet or self.minter_id
        require!(
            sender_wallet_id == self.minter_id
                || sender_wallet_id == self.wallet_account_id(&sender_id),
            "invalid wallet",
        );

        self.balance.0 = self
            .balance
            .0
            .checked_add(amount.0)
            .unwrap_or_else(|| env::panic_str("balance overflow"));

        let Some(notify) = notify else {
            // no transfer notification, all tokens received
            return PromiseOrValue::Value(amount);
        };

        let forward_deposit = {
            let state_init_amount =
                notify.state_init.as_ref().map(|s| s.amount).unwrap_or_default();

            env::attached_deposit()
                .checked_sub(state_init_amount)
                .unwrap_or_else(|| env::panic_str("insufficient attached deposit"))
        };

        sft_receiver::ext(self.owner_id.clone())
            // forward all attached deposit
            .with_attached_deposit(forward_deposit)
            // forward all remaining gas here
            .with_unused_gas_weight(1)
            .with_state_init(notify.state_init)
            .sft_on_transfer(sender_id, amount, notify.msg)
            .then(
                Self::ext(env::current_account_id())
                    .with_static_gas(Self::SFT_RESOLVE_GAS)
                    .with_unused_gas_weight(0)
                    .sft_resolve(amount, false),
            )
            .into()
    }

    /// Code of this wallet-contract will be re-used across all applications
    /// that want to interact with sharded fungible tokens, so we need a
    /// uniform method to burn tokens to be supported by every wallet-contract.
    /// If the minter-contract doesn't support burning, these tokens
    /// will be minted back on burner wallet-contract.
    ///
    /// Returns `burned_amount`
    #[payable]
    fn sft_burn(&mut self, amount: U128, msg: String) -> PromiseOrValue<U128> {
        assert_at_least_one_yocto_near();
        require!(env::predecessor_account_id() == self.owner_id, "not owner");

        self.balance.0 = self
            .balance
            .0
            .checked_sub(amount.0)
            .unwrap_or_else(|| env::panic_str("insufficient balance"));

        sft_burner::ext(self.minter_id.clone())
            // forward all attached deposit
            .with_attached_deposit(env::attached_deposit())
            // forward all remaining gas here
            .with_unused_gas_weight(1)
            .sft_on_burn(self.owner_id.clone(), amount, msg)
            .then(
                Self::ext(env::current_account_id())
                    .with_static_gas(Self::SFT_RESOLVE_GAS)
                    // do not distribute remaining gas here
                    .with_unused_gas_weight(0)
                    .sft_resolve(amount, true),
            )
            .into()
    }

    /// Gets result from `sft_receive()`, `sft_on_transfer()`
    /// or `sft_on_burn()`, adjusts the balance accordingly
    /// and returns `used_amount`.
    #[private]
    fn sft_resolve(&mut self, amount: U128, sender: bool) -> U128 {
        // FIXME: NEP-141 return-bomb vulnerability is still here,
        // we need to check `env::promise_result_length(0)` first.
        // Otherwise, failing `receiver_wallet_id::sft_resolve()`
        // results in double spend: tokens are not subtracted on
        // `receiver_wallet_id`, but refunded for `receiver_wallet_id`.
        let mut used_amount = match env::promise_result(0) {
            PromiseResult::Successful(value) => {
                serde_json::from_slice::<U128>(&value).unwrap_or_default().0.min(amount.0)
            }
            PromiseResult::Failed => amount.0,
        };

        let mut refund_amount = amount.0.saturating_sub(used_amount);

        self.balance.0 = if sender {
            // add refund_amount to sender, but in checked way:
            // faulty minter-contract implementation could have minted
            // too many tokens after `sft_resolve_transfer()` was scheduled
            // but not executed yet
            self.balance
                .0
                .checked_add(refund_amount)
                .unwrap_or_else(|| env::panic_str("balance overflow"))
        } else {
            // refund maximum what we can
            refund_amount = refund_amount.min(self.balance.0);
            // update used_amount
            used_amount = amount.0.saturating_sub(refund_amount);
            // subtract refund from receiver
            self.balance.0.saturating_sub(refund_amount)
        };

        U128(used_amount)
    }
}

impl ShardedFungibleTokenWalletData {
    /// Deteministically derive account_id of wallet-contract
    /// for given `owner_id`
    fn wallet_account_id(&self, owner_id: &AccountIdRef) -> AccountId {
        self.state_init_for(owner_id).derived_account_id()
    }

    /// Prepare `StateInit` for wallet-contract of `self.minter_id` for given `owner_id`
    fn state_init_for(&self, owner_id: &AccountIdRef) -> StateInit {
        StateInit {
            code: env::current_contract_code(),
            init_call: Some(StateInitFunctionCall {
                function_name: "init".to_string(),
                args: borsh::to_vec(&InitArgs {
                    owner_id: Cow::Borrowed(owner_id),
                    minter_id: Cow::Borrowed(&self.minter_id),
                })
                .unwrap()
                .into(),
            }),
        }
    }
}

#[inline]
#[track_caller]
fn assert_at_least_one_yocto_near() {
    require!(
        env::attached_deposit() >= NearToken::from_yoctonear(1),
        "required attached deposit of at least 1 yoctoNEAR"
    );
}
