#![allow(clippy::too_many_arguments)]

use std::borrow::Cow;

use near_sdk::{
    ext_contract,
    json_types::U128,
    near,
    serde_with::{serde_as, DisplayFromStr},
    AccountId, AccountIdRef, ContractStorage, Gas, LazyStateInit, NearToken, PromiseOrValue,
};

use crate::contract_state::ContractState;

/// # Sharded Fungible Token wallet-contract
///
/// This is a contract that holds:
/// * owner's balance
/// * owner's [`AccountId`]
/// * [minter](super::minter::ShardedFungibleTokenMinter)'s [`AccountId`]
#[ext_contract(ext_sft_wallet)]
pub trait ShardedFungibleTokenWallet {
    /// View method to get all data at once
    fn sft_wallet_data(self) -> ContractState<SFTWalletData<'static>>;

    /// Transfer given `amount` of tokens to `receiver_id`.
    ///
    /// Unless `no_init` is set, requires additional attached deposit to pay for
    /// automatic deployment and initialization of receiver's wallet-contract.
    /// If by the time the created receipt gets executed it turns out to be
    /// already deployed, then reserved NEAR tokens refunded to `refund_to` or
    /// predecessor, if not set. See [`near_sdk::StateInit`] and
    /// [`near_sdk::env::promise_set_refund_to()`].
    ///
    /// If `notify` is set, then [`receiver_id::sft_on_transfer()`](super::receiver::ShardedFungibleTokenReceiver)
    /// will be called. If `notify.state_init` is set, then `receiver_id` will
    /// be initialized if doesn't exist. See [`TransferNotification`].
    ///
    /// Returns `used_amount`.
    ///
    /// In order to allow for more flexibity while having a common interface,
    /// implementations MAY use `custom_payload` for extended functionality,
    /// such as [mintless tokens](https://github.com/ton-blockchain/mintless-jetton-contract).
    ///
    /// Note: MUST be `#[payable]` and require at least 1yN attached.
    fn sft_transfer(
        &mut self,
        receiver_id: AccountId,
        amount: U128,
        custom_payload: Option<String>,
        memo: Option<String>,
        notify: Option<TransferNotification>,
        refund_to: Option<AccountId>,
        no_init: Option<bool>,
    ) -> PromiseOrValue<U128>;

    /// Receives tokens from [minter-contract](super::minter::ShardedFungibleTokenMinter)
    /// or wallet-contracts initialized for the same minter-contract.
    ///
    /// If `notify` is set, then `receiver_id::sft_on_transfer()` will be
    /// called. If `notify.state_init` is set, then `receiver_id` will be
    /// initialized if doesn't exist.
    ///
    /// Remaining attached deposit is refunded to `refund_to` or `sender_id`
    /// if not set.
    ///
    /// Returns `used_amount`.
    ///
    /// Note: MUST be `#[payable]` and require at least 1yN attached.
    fn sft_receive(
        &mut self,
        sender_id: AccountId,
        amount: U128,
        memo: Option<String>,
        notify: Option<TransferNotification>,
        refund_to: Option<AccountId>,
    ) -> PromiseOrValue<U128>;

    /// Burn given `amount` and notify [`minter_id::sft_on_burn()`](super::minter::ShardedFungibleTokenBurner::sft_on_burn).
    /// If `minter_id` doesn't support burning or returns partial
    /// `used_amount`, then `amount - used_amount` will be minter back
    /// on `sender_id`.
    ///
    /// Code of this wallet-contract will be re-used across all applications
    /// that want to interact with sharded fungible tokens, so we need a
    /// uniform method to burn tokens to be supported by every wallet-contract.
    /// If the minter-contract doesn't support burning, these tokens
    /// will be minted back on burner wallet-contract.
    ///
    /// Returns `burned_amount`.
    ///
    /// In order to allow for more flexibity while having a common interface,
    /// implementations MAY use `custom_payload` for extended functionality,
    /// such as [mintless tokens](https://github.com/ton-blockchain/mintless-jetton-contract).
    ///
    /// Note: MUST be `#[payable]` and require at least 1yN attached
    fn sft_burn(
        &mut self,
        amount: U128,
        custom_payload: Option<String>,
        msg: String,
    ) -> PromiseOrValue<U128>;
}

/// Sharded Fungible Token wallet-contract data
#[near(serializers = [borsh, json])]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SFTWalletData<'a> {
    /// Optional status to be used by extended implementations, such as
    /// [governed tokens](https://github.com/ton-blockchain/stablecoin-contract/tree/main), or for future upgrades.
    pub status: u8,

    /// Balance of the owner
    #[serde_as(as = "DisplayFromStr")]
    pub balance: u128,

    /// Owner's [`AccountId`]
    pub owner_id: Cow<'a, AccountIdRef>,

    /// [Minter](super::minter::ShardedFungibleTokenMinter)'s [`AccountId`]
    pub minter_id: Cow<'a, AccountIdRef>,
}

impl<'a> SFTWalletData<'a> {
    pub const STATE_KEY: &'static [u8] = b"";
    // TODO: calculate exact values
    pub const SFT_RECEIVE_MIN_GAS: Gas = Gas::from_tgas(5);
    pub const SFT_RESOLVE_GAS: Gas = Gas::from_tgas(5);

    #[inline]
    pub fn init(
        owner_id: impl Into<Cow<'a, AccountIdRef>>,
        minter_id: impl Into<Cow<'a, AccountIdRef>>,
    ) -> Self {
        Self { status: 0, balance: 0, owner_id: owner_id.into(), minter_id: minter_id.into() }
    }

    #[inline]
    pub fn init_state(
        owner_id: impl Into<Cow<'a, AccountIdRef>>,
        minter_id: impl Into<Cow<'a, AccountIdRef>>,
    ) -> ContractStorage {
        ContractStorage::new().borsh(Self::STATE_KEY, Self::init(owner_id, minter_id))
    }
}

/// Arguments for constructing [`receiver_id::sft_on_receive()`](super::receiver::ShardedFungibleTokenReceiver::sft_on_receive) notification.
#[near(serializers = [borsh, json])]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransferNotification {
    /// Optionally, deploy & init `receiver_id` contract if didn't exist.
    /// It enables for better composability when transferring to other
    /// not-yet-initialized owner contracts.
    #[serde(flatten, default, skip_serializing_if = "Option::is_none")]
    pub state_init: Option<StateInitArgs>,

    /// Message to pass in [`receiver_id::sft_on_receive()`](super::receiver::ShardedFungibleTokenReceiver::sft_on_receive)
    pub msg: String,

    /// Amount of NEAR tokens to attach to `[`receiver_id::sft_on_receive()`](super::receiver::ShardedFungibleTokenReceiver::sft_on_receive)
    /// call.
    #[serde(default, skip_serializing_if = "NearToken::is_zero")]
    pub forward_deposit: NearToken,
}

impl TransferNotification {
    #[inline]
    pub fn msg(msg: String) -> Self {
        Self { state_init: None, msg, forward_deposit: NearToken::from_yoctonear(0) }
    }

    #[inline]
    pub fn state_init(mut self, state_init: LazyStateInit, amount: NearToken) -> Self {
        self.state_init = Some(StateInitArgs { state_init, state_init_amount: amount });
        self
    }

    #[inline]
    pub fn forward_deposit(mut self, amount: NearToken) -> Self {
        self.forward_deposit = amount;
        self
    }
}

#[near(serializers=[borsh, json])]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StateInitArgs {
    pub state_init: LazyStateInit,

    #[serde(default, skip_serializing_if = "NearToken::is_zero")]
    pub state_init_amount: NearToken,
}

/// # Governed Sharded Fungible Token wallet-contract
///
/// Same as [ShardedFungibleTokenWallet], but
/// [minter-contract](super::minter::ShardedFungibleTokenMinter) is also
/// allowed to:
/// * forcily transfer by calling [`.sft_transfer()`](ShardedFungibleTokenWallet::sft_transfer)
/// * lock outgoing transfers
/// * lock incoming transfers
#[ext_contract(ext_sft_wallet_governed)]
pub trait ShardedFungibleTokenWalletGoverned: ShardedFungibleTokenWallet {
    /// Set status (only allowed for minter).
    ///
    /// Note: MUST have exactly 1yN attached.
    fn sft_wallet_set_status(&mut self, status: u8);
}

#[cfg(test)]
mod tests {
    use near_sdk::{ContractCode, StateInit};

    use super::*;

    #[test]
    fn storage_cost() {
        let long_account_id: AccountId = "a".repeat(64).parse().unwrap();

        let code = ContractCode::GlobalAccountId("wallet.sft.near".parse().unwrap());
        let data = SFTWalletData::init_state(&long_account_id, &long_account_id);

        let state_init = StateInit::code(code).data(data);

        println!("max storage cost: {}", state_init.storage_cost());
    }
}
