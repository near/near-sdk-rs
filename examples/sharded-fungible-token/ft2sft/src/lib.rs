use impl_tools::autoimpl;
use near_contract_standards::{
    fungible_token::{core::ext_ft_core, receiver::FungibleTokenReceiver},
    sharded_fungible_token::{
        minter::{
            SftMinterData, ShardedFungibleTokenBurner, ShardedFungibleTokenMinter,
            ft2sft::{BurnMessage, Ft2Sft, Ft2SftData, MintMessage},
        },
        wallet::{SftWalletData, ext_sft_wallet},
    },
    storage_management::ext_storage_management,
};
use near_sdk::{
    AccountId, AccountIdRef, Gas, NearToken, PanicOnDefault, Promise, PromiseError, PromiseOrValue,
    env,
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

        let receiver_id = mint.receiver_id.as_deref().unwrap_or(&sender_id);
        let sft_wallet_id = self.sft_wallet_account_id_for(receiver_id);

        // We don't do `.with_state_init()`, since there is no way to attach
        // deposit to `.ft_on_transfer()` according to NEP-141 spec. So, if
        // the wallet-contract turns out to *not* exist, we will refund NEP-141
        // tokens to sender in `.resolve_transfer()`. Thus, if sender wants to
        // ensure successful wrapping, he needs to create receiver's
        // wallet-contract by himself in advance.
        //
        // It doesn't make sense to expose additional function for creation of
        // wallet-contracts (e.g. `.storage_deposit()`), since the caller's
        // intention is to further interact with receiver's wallet-contract,
        // so he needs a way to calculate its account id, and, thus, can
        // deploy & init it by himself.
        ext_sft_wallet::ext(sft_wallet_id)
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
                    .resolve_transfer(Op::SftReceive, amount),
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
        let mut deposit_left = env::attached_deposit()
            .checked_sub(NearToken::from_yoctonear(1))
            .unwrap_or_else(|| env::panic_str(ERR_INSUFFICIENT_DEPOSIT));

        require!(
            env::predecessor_account_id()
                == self.sft_wallet_account_id_for(&env::current_account_id()),
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

        let receiver_id = burn.receiver_id.unwrap_or_else(|| sender_id.clone());

        let mut p = Promise::new(self.ft_contract_id.clone());

        if !burn.storage_deposit.is_zero() {
            deposit_left = deposit_left
                .checked_sub(burn.storage_deposit)
                .unwrap_or_else(|| env::panic_str(ERR_INSUFFICIENT_DEPOSIT));

            p = ext_storage_management::ext_on(p)
                .with_attached_deposit(burn.storage_deposit)
                .with_static_gas(Self::STORAGE_DEPOSIT_GAS)
                // do not distribute remaining gas here
                .with_unused_gas_weight(0)
                .storage_deposit(Some(receiver_id.clone()), None);
        }

        if !deposit_left.is_zero() {
            // detached
            let _ = Promise::new(burn.refund_to.unwrap_or(sender_id)).transfer(deposit_left);
        }

        let op: Op;
        let ft_ext = ext_ft_core::ext_on(p)
            // both `.ft_transfer()` and `.ft_transfer_call()` require 1yN
            .with_attached_deposit(NearToken::from_yoctonear(1))
            // forward here all remaining gas
            .with_unused_gas_weight(1);
        (op, p) = if let Some(msg) = burn.msg {
            (
                Op::FtTransferCall,
                ft_ext
                    // require minimum gas
                    .with_static_gas(Self::FT_TRANSFER_CALL_MIN_GAS)
                    .ft_transfer_call(receiver_id, amount, burn.memo, msg),
            )
        } else {
            (
                Op::FtTransfer,
                ft_ext
                    // require minimum gas
                    .with_static_gas(Self::FT_TRANSFER_MIN_GAS)
                    .ft_transfer(receiver_id, amount, burn.memo),
            )
        };

        p.then(
            Self::ext(env::current_account_id())
                .with_static_gas(Self::RESOLVE_TRANSFER_GAS)
                // do not distribute remaining gas here
                .with_unused_gas_weight(0)
                .resolve_transfer(op, amount),
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
    pub fn resolve_transfer(&mut self, op: Op, amount: U128) -> U128 {
        let used_amount = op
            .extract_used_amount(env::promise_result_checked(0, Self::MAX_RESULT_LENGTH), amount.0);

        let unused_amount = amount.0.saturating_sub(used_amount);

        match op {
            Op::SftReceive => {
                // subtract from total_supply
                self.total_supply = self
                    .total_supply
                    .checked_sub(unused_amount)
                    .unwrap_or_else(|| env::panic_str(ERR_SUPPLY_OVERFLOW));

                // return unused amount from `self::ft_on_transfer()`
                U128(unused_amount)
            }
            Op::FtTransfer | Op::FtTransferCall => {
                // add back to total_supply
                self.total_supply = self
                    .total_supply
                    .checked_add(unused_amount)
                    .unwrap_or_else(|| env::panic_str(ERR_SUPPLY_OVERFLOW));

                // return used amount from `self::sft_on_burn()`
                U128(used_amount)
            }
        }
    }
}

impl Ft2SftContract {
    fn sft_wallet_init_for(&self, owner_id: &AccountIdRef) -> StateInit {
        StateInit::V1(StateInitV1 {
            code: self.sft_wallet_code.clone(),
            data: SftWalletData::init_state(owner_id, env::current_account_id()),
        })
    }

    fn sft_wallet_account_id_for(&self, owner_id: &AccountIdRef) -> AccountId {
        self.sft_wallet_init_for(owner_id).derive_account_id()
    }
}

#[near(serializers = [json])]
#[serde(rename_all = "snake_case")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Op {
    SftReceive,
    FtTransfer,
    FtTransferCall,
}

impl Op {
    fn extract_used_amount(self, result: Result<Vec<u8>, PromiseError>, sent: u128) -> u128 {
        match result {
            Ok(data) => match self {
                Self::SftReceive | Self::FtTransferCall => {
                    // both `sft_receive` and `ft_transfer_call` return used amount
                    serde_json::from_slice::<U128>(&data).unwrap_or_default().0.min(sent)
                }
                Self::FtTransfer => {
                    // `ft_transfer` returns empty result on success
                    if data.is_empty() { sent } else { 0 }
                }
            },
            _ => match self {
                Self::SftReceive | Self::FtTransfer => 0,
                // do not refund on failed `ft_transfer_call` due to
                // NEP-141 vulnerability: `ft_resolve_transfer` fails to
                // read result of `ft_on_transfer` due to insufficient gas
                Self::FtTransferCall => sent,
            },
        }
    }
}

const ERR_WRONG_TOKEN: &str = "wrong token";
const ERR_WRONG_WALLET: &str = "wrong wallet";
const ERR_SUPPLY_OVERFLOW: &str = "total_supply overflow";
const ERR_INVALID_JSON: &str = "invalid JSON";
const ERR_INSUFFICIENT_DEPOSIT: &str = "insufficient attached deposit";
