use impl_tools::autoimpl;
use near_contract_standards::{
    fungible_token::{core::ext_ft_core, receiver::FungibleTokenReceiver},
    sharded_fungible_token::{
        minter::{
            SftMinterData, ShardedFungibleTokenBurner, ShardedFungibleTokenMinter,
            ft2sft::{BurnMessage, Ft2Sft, Ft2SftData, MintMessage},
        },
        wallet::{
            SftWalletData,
            events::{SftBurn, SftEvent, SftMint},
            ext_sft_wallet,
        },
    },
    storage_management::ext_storage_management,
};
use near_sdk::{
    AccountId, Gas, NearToken, PanicOnDefault, Promise, PromiseOrValue, env,
    json_types::U128,
    near, require, serde_json,
    state_init::{StateInit, StateInitV1},
};

/// Reference implementation for
/// [Fungible Tokens to Sharded Fungible Tokens adaptor](Ft2Sft)
#[near(contract_state(key = Ft2SftData::STATE_KEY))]
#[autoimpl(Deref using self.0)]
#[autoimpl(DerefMut using self.0)]
#[derive(PanicOnDefault)]
#[repr(transparent)]
struct Ft2SftContract(Ft2SftData);

#[near]
impl Ft2Sft for Ft2SftContract {
    fn ft_contract_id(self) -> AccountId {
        self.0.ft_contract_id
    }
}

#[near]
impl ShardedFungibleTokenMinter for Ft2SftContract {
    fn sft_minter_data(self) -> SftMinterData {
        self.0.data
    }

    fn sft_wallet_account_id_for(&self, owner_id: AccountId) -> AccountId {
        self.sft_wallet_account_id_for(&owner_id)
    }
}

#[near]
impl FungibleTokenReceiver for Ft2SftContract {
    /// Mint (i.e. "wrap") received fungible tokens into sharded ones.
    fn ft_on_transfer(
        &mut self,
        sender_id: AccountId,
        amount: U128,
        msg: String,
    ) -> PromiseOrValue<U128> {
        require!(env::predecessor_account_id() == *self.ft_contract_id, ERR_WRONG_TOKEN);

        let mint: MintMessage = if msg.is_empty() {
            MintMessage::default()
        } else {
            serde_json::from_str(&msg).unwrap_or_else(|_| env::panic_str(ERR_INVALID_JSON))
        };

        self.total_supply = self
            .total_supply
            .checked_add(amount.0)
            .unwrap_or_else(|| env::panic_str(ERR_SUPPLY_OVERFLOW));

        let receiver_id = mint.receiver_id.unwrap_or(sender_id.clone());

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
                // refund attached deposit in case of failure to `refund_to`
                // or sender_id
                .refund_to(mint.refund_to.as_deref().unwrap_or(&sender_id))
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
impl ShardedFungibleTokenBurner for Ft2SftContract {
    /// Burn sharded fungible tokens and unwrap into non-sharded ones.
    #[payable]
    fn sft_on_burn(
        &mut self,
        sender_id: AccountId,
        amount: U128,
        msg: String,
    ) -> PromiseOrValue<U128> {
        let deposit_left = env::attached_deposit()
            // reserve 1yN for `ft_transfer()` / `ft_trnsfer_call()` later
            .checked_sub(NearToken::from_yoctonear(1))
            .unwrap_or_else(|| env::panic_str(ERR_INSUFFICIENT_DEPOSIT));

        require!(
            env::predecessor_account_id() == self.sft_wallet_account_id_for(&sender_id),
            ERR_WRONG_WALLET,
        );

        let burn: BurnMessage = if msg.is_empty() {
            BurnMessage::default()
        } else {
            serde_json::from_str(&msg).unwrap_or_else(|_| env::panic_str(ERR_INVALID_JSON))
        };

        self.total_supply = self
            .total_supply
            .checked_sub(amount.0)
            .unwrap_or_else(|| env::panic_str(ERR_SUPPLY_OVERFLOW));

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

        let p = ext_ft_core::ext_on(if burn.storage_deposit.is_zero() {
            Promise::new(self.ft_contract_id.clone())
        } else {
            // make sure enough deposit is attached
            require!(deposit_left >= burn.storage_deposit, ERR_INSUFFICIENT_DEPOSIT);

            ext_storage_management::ext(self.ft_contract_id.clone())
                .with_attached_deposit(burn.storage_deposit)
                .with_static_gas(Self::STORAGE_DEPOSIT_GAS)
                // do not distribute remaining gas here
                .with_unused_gas_weight(0)
                .storage_deposit(Some(receiver_id.clone()), None)
        })
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
impl Ft2SftContract {
    const STORAGE_DEPOSIT_GAS: Gas = Gas::from_tgas(10);
    const FT_TRANSFER_MIN_GAS: Gas = Gas::from_tgas(10);
    const FT_TRANSFER_CALL_MIN_GAS: Gas = Gas::from_tgas(30);
    const RESOLVE_TRANSFER_GAS: Gas = Gas::from_tgas(5);

    #[allow(clippy::as_conversions)]
    const MAX_RESULT_LENGTH: usize = "\"+340282366920938463463374607431768211455\"".len(); // u128::MAX

    #[allow(dead_code)]
    #[private]
    pub fn sft_resolve_mint(&mut self, owner_id: AccountId, amount: U128) -> U128 {
        let minted_amount = env::promise_result_checked(0, Self::MAX_RESULT_LENGTH)
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
                .unwrap_or_else(|| env::panic_str(ERR_SUPPLY_OVERFLOW));

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

    #[allow(dead_code)]
    #[private]
    pub fn sft_resolve_burn(&mut self, owner_id: AccountId, amount: U128, is_call: bool) -> U128 {
        let burned_amount = match env::promise_result_checked(0, Self::MAX_RESULT_LENGTH) {
            Ok(data) => {
                if is_call {
                    // `ft_transfer_call` returns used amount
                    serde_json::from_slice::<U128>(&data).unwrap_or_default().0.min(amount.0)
                } else if data.is_empty() {
                    // `ft_transfer` returns empty result on success
                    amount.0
                } else {
                    0
                }
            }
            Err(_) => {
                if is_call {
                    // do not refund on failed `ft_transfer_call` due to
                    // NEP-141 vulnerability: `ft_resolve_transfer` fails to
                    // read result of `ft_on_transfer` due to insufficient gas
                    amount.0
                } else {
                    0
                }
            }
        };

        let mint_amount = amount.0.saturating_sub(burned_amount);
        if mint_amount > 0 {
            // add back to total_supply
            self.total_supply = self
                .total_supply
                .checked_add(mint_amount)
                .unwrap_or_else(|| env::panic_str(ERR_SUPPLY_OVERFLOW));

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

impl Ft2SftContract {
    fn sft_wallet_init_for(&self, owner_id: impl Into<AccountId>) -> StateInit {
        StateInit::V1(StateInitV1 {
            code: self.sft_wallet_code.clone(),
            data: SftWalletData::init_state(owner_id, env::current_account_id()),
            // TODO: governed ft2sft proxy?
        })
    }

    fn sft_wallet_account_id_for(&self, owner_id: impl Into<AccountId>) -> AccountId {
        self.sft_wallet_init_for(owner_id).derive_account_id()
    }
}

const ERR_WRONG_TOKEN: &str = "wrong token";
const ERR_WRONG_WALLET: &str = "wrong wallet";
const ERR_SUPPLY_OVERFLOW: &str = "total_supply overflow";
const ERR_INVALID_JSON: &str = "invalid JSON";
const ERR_INSUFFICIENT_DEPOSIT: &str = "insufficient attached deposit";
