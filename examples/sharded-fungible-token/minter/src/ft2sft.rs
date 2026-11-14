///! Reference implementation for Fungible Token to Sharded Fungible Token adaptor
///! It mints sharded fungible tokens on [`.ft_on_transfer()`](FungibleTokenReceiver::ft_on_transfer)
///! and burns them back in [`.sft_on_burn()`](ShardedFungibleTokenBurner::sft_on_burn).
use near_contract_standards::{
    fungible_token::{core::ext_ft_core, receiver::FungibleTokenReceiver},
    sharded_fungible_token::{
        events::{SftBurn, SftEvent, SftMint},
        minter::{
            ShardedFungibleTokenBurner,
            ft2sft::{BurnMessage, MintMessage},
        },
        wallet::{SftWalletData, ext_sft_wallet},
    },
    storage_management::ext_storage_management,
};
use near_sdk::{
    AccountId, Gas, NearToken, Promise, PromiseOrValue, env, json_types::U128, near, require,
    serde_json,
};

use crate::{Contract, ContractExt};

#[near]
impl FungibleTokenReceiver for Contract {
    /// Mint (i.e. "wrap") received fungible tokens into sharded ones.
    fn ft_on_transfer(
        &mut self,
        sender_id: AccountId,
        amount: U128,
        msg: String,
    ) -> PromiseOrValue<U128> {
        require!(env::predecessor_account_id() == *self.authority_id, Self::ERR_WRONG_TOKEN);
        require!(amount.0 > 0, Self::ERR_ZERO_AMOUNT);

        let mint: MintMessage = if msg.is_empty() {
            MintMessage::default()
        } else {
            serde_json::from_str(&msg).unwrap_or_else(|_| env::panic_str(Self::ERR_INVALID_JSON))
        };

        self.total_supply = self
            .total_supply
            .checked_add(amount.0)
            .unwrap_or_else(|| env::panic_str(Self::ERR_SUPPLY_OVERFLOW));

        let receiver_id = mint.receiver_id.unwrap_or_else(|| sender_id.clone());

        SftEvent::Mint(
            [SftMint {
                owner_id: (&receiver_id).into(),
                amount: amount.0,
                memo: mint.memo.as_deref().map(Into::into),
            }]
            .as_slice()
            .into(),
        )
        .emit();

        ext_sft_wallet::ext_on({
            let state_init = self.sft_wallet_init_for(&receiver_id);
            Promise::new(state_init.derive_account_id())
                // always deploy & init receiver's wallet-contract
                .state_init(
                    state_init,
                    // sFT wallet-contract fits into ZBA limits, i.e. < 770 bytes
                    NearToken::ZERO,
                )
        })
        // Attach 1yN according to `.sft_receive()` specification. Draining
        // this minter-contract is not profitable for an attacker, since
        // gas per mint is more expensive than the potential profit of 1yN.
        // If we run out of NEAR, then anyone can replenish this account
        // permissionlessly. Moreover, according to Near specs, 30% of gas
        // goes to the contract balance, so we definitely have it.
        .with_attached_deposit(NearToken::from_yoctonear(1))
        .with_static_gas(SftWalletData::SFT_RECEIVE_MIN_GAS)
        // forward remaining gas here
        .with_unused_gas_weight(1)
        .sft_receive(
            // Note: there is no guarantee that `sender_id` from
            // `.ft_on_transfer()` indeed initiated the transfer.
            sender_id,
            amount,
            mint.memo,
            mint.notify,
        )
        .then(
            Self::ext(env::current_account_id())
                .with_static_gas(Self::RESOLVE_TRANSFER_GAS)
                // do not distribute remaining gas here
                .with_unused_gas_weight(0)
                .sft_resolve_mint(receiver_id, amount),
        )
        .into()
    }
}

#[near]
impl ShardedFungibleTokenBurner for Contract {
    /// Burn sharded fungible tokens and unwrap into non-sharded ones.
    #[payable]
    fn sft_on_burn(
        &mut self,
        sender_id: AccountId,
        amount: U128,
        msg: String,
    ) -> PromiseOrValue<U128> {
        require!(amount.0 > 0, Self::ERR_ZERO_AMOUNT);
        let deposit_left = env::attached_deposit()
            // reserve 1yN for `ft_transfer()` / `ft_transfer_call()` later
            .checked_sub(NearToken::from_yoctonear(1))
            .unwrap_or_else(|| env::panic_str(Self::ERR_INSUFFICIENT_DEPOSIT));

        require!(
            env::predecessor_account_id() == self.sft_wallet_account_id_for(&sender_id),
            Self::ERR_WRONG_WALLET,
        );

        let burn: BurnMessage = if msg.is_empty() {
            BurnMessage::default()
        } else {
            serde_json::from_str(&msg).unwrap_or_else(|_| env::panic_str(Self::ERR_INVALID_JSON))
        };

        self.total_supply = self
            .total_supply
            .checked_sub(amount.0)
            .unwrap_or_else(|| env::panic_str(Self::ERR_SUPPLY_OVERFLOW));

        SftEvent::Burn(
            [SftBurn {
                owner_id: (&sender_id).into(),
                amount: amount.0,
                memo: burn.memo.as_deref().map(Into::into),
            }]
            .as_slice()
            .into(),
        )
        .emit();

        let receiver_id = burn.receiver_id.unwrap_or_else(|| sender_id.clone());

        let mut p = Promise::new(self.authority_id.clone())
            // refund storage_deposit (if any) + 1yN in case of failure to
            // `refund_to` set by burner (or predecessor, otherwise) instead of
            // current ft2sft
            .refund_to(env::refund_to_account_id());

        // if more than 1yN was attached
        if !deposit_left.is_zero() {
            // pay the rest for storage_deposit
            p = ext_storage_management::ext_on(p)
                .with_attached_deposit(deposit_left)
                .with_static_gas(Self::STORAGE_DEPOSIT_GAS)
                // do not distribute remaining gas here
                .with_unused_gas_weight(0)
                .storage_deposit(Some(receiver_id.clone()), None);
        }

        let p = ext_ft_core::ext_on(p)
            // both `.ft_transfer()` and `.ft_transfer_call()` require 1yN
            .with_attached_deposit(NearToken::from_yoctonear(1))
            // forward here all remaining gas
            .with_unused_gas_weight(1);

        let is_call = burn.msg.is_some();
        if let Some(msg) = burn.msg {
            p.with_static_gas(Self::FT_TRANSFER_CALL_MIN_GAS).ft_transfer_call(
                receiver_id,
                amount,
                burn.memo,
                msg,
            )
        } else {
            p.with_static_gas(Self::FT_TRANSFER_MIN_GAS).ft_transfer(receiver_id, amount, burn.memo)
        }
        .then(
            Self::ext(env::current_account_id())
                .with_static_gas(Self::RESOLVE_TRANSFER_GAS)
                // do not distribute remaining gas here
                .with_unused_gas_weight(0)
                .sft_resolve_burn(sender_id, amount, is_call),
        )
        .into()
    }
}

#[near]
impl Contract {
    const STORAGE_DEPOSIT_GAS: Gas = Gas::from_tgas(10);
    const FT_TRANSFER_MIN_GAS: Gas = Gas::from_tgas(10);
    const FT_TRANSFER_CALL_MIN_GAS: Gas = Gas::from_tgas(30);
    const RESOLVE_TRANSFER_GAS: Gas = Gas::from_tgas(5);

    const MAX_RESULT_LENGTH: usize = "\"+340282366920938463463374607431768211455\"".len(); // u128::MAX

    #[private]
    pub fn sft_resolve_mint(&mut self, owner_id: AccountId, amount: U128) -> U128 {
        let minted_amount = env::promise_result_checked(
            0,
            Self::MAX_RESULT_LENGTH, // prevent out of gas (too long result)
        )
        .ok() // promise failed or result was too long
        .and_then(|data| serde_json::from_slice::<U128>(&data).ok()) // JSON
        .unwrap_or_default()
        .0
        .min(amount.0);

        let burn_amount = amount.0.saturating_sub(minted_amount);
        if burn_amount > 0 {
            self.total_supply = self
                .total_supply
                .checked_sub(burn_amount)
                .unwrap_or_else(|| env::panic_str(Self::ERR_SUPPLY_OVERFLOW));

            SftEvent::Burn(
                [SftBurn {
                    owner_id: owner_id.into(),
                    amount: burn_amount,
                    memo: Some("refund".into()),
                }]
                .as_slice()
                .into(),
            )
            .emit();
        }

        // return unused amount from `self::ft_on_transfer()`
        U128(burn_amount)
    }

    #[private]
    pub fn sft_resolve_burn(&mut self, owner_id: AccountId, amount: U128, is_call: bool) -> U128 {
        let burned_amount = env::promise_result_checked(
            0,
            Self::MAX_RESULT_LENGTH, // prevent out of gas (too long result)
        )
        .map_or(
            if is_call {
                // do not refund on failed `ft_transfer_call` due to
                // NEP-141 vulnerability: `ft_resolve_transfer` fails to
                // read result of `ft_on_transfer` due to insufficient gas
                amount.0
            } else {
                0
            },
            |data| {
                if is_call {
                    // `ft_transfer_call` returns used amount
                    serde_json::from_slice::<U128>(&data).unwrap_or_default().0.min(amount.0)
                } else if data.is_empty() {
                    // `ft_transfer` returns empty result on success
                    amount.0
                } else {
                    0
                }
            },
        );

        let mint_amount = amount.0.saturating_sub(burned_amount);
        if mint_amount > 0 {
            // add back to total_supply
            self.total_supply = self
                .total_supply
                .checked_add(mint_amount)
                .unwrap_or_else(|| env::panic_str(Self::ERR_SUPPLY_OVERFLOW));

            SftEvent::Mint(
                [SftMint {
                    owner_id: owner_id.into(),
                    amount: mint_amount,
                    memo: Some("refund".into()),
                }]
                .as_slice()
                .into(),
            )
            .emit();
        }

        // return used amount from `self::sft_on_burn()`
        U128(burned_amount)
    }
}

impl Contract {
    const ERR_WRONG_TOKEN: &str = "wrong token";
    const ERR_WRONG_WALLET: &str = "wrong wallet";
    const ERR_ZERO_AMOUNT: &str = "zero amount";
    const ERR_SUPPLY_OVERFLOW: &str = "total_supply overflow";
    const ERR_INVALID_JSON: &str = "invalid JSON";
}
