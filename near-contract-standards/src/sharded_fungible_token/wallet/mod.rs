pub mod events;
pub mod governed;

use std::collections::BTreeMap;

use near_sdk::{
    borsh, ext_contract,
    json_types::U128,
    near,
    serde_with::{serde_as, DisplayFromStr},
    state_init::StateInit,
    AccountId, Gas, NearToken, PromiseOrValue,
};

/// # Sharded Fungible Token wallet-contract
///
/// This is a contract that holds:
/// * owner's balance
/// * owner's [`AccountId`]
/// * [minter](super::minter::ShardedFungibleTokenMinter)'s [`AccountId`]
#[ext_contract(ext_sft_wallet)]
pub trait ShardedFungibleTokenWallet {
    /// View method to get all data at once
    fn sft_wallet_data(self) -> SftWalletData;

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
    /// Note: MUST be `#[payable]` and require at least 1yN attached.
    fn sft_send(
        &mut self,
        receiver_id: AccountId,
        amount: U128,
        memo: Option<String>,
        notify: Option<TransferNotification>,
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
    /// Note: MUST be `#[payable]` and require at least 1yN attached
    fn sft_burn(&mut self, amount: U128, memo: Option<String>, msg: String)
        -> PromiseOrValue<U128>;
}

/// Sharded Fungible Token wallet-contract data
#[near(serializers = [borsh, json])]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SftWalletData {
    /// [Minter](super::minter::ShardedFungibleTokenMinter)'s [`AccountId`]
    pub minter_id: AccountId,

    /// Owner's [`AccountId`]
    pub owner_id: AccountId,

    /// Balance of the owner
    #[serde_as(as = "DisplayFromStr")]
    pub balance: u128,
}

impl SftWalletData {
    pub const STATE_KEY: &'static [u8] = b"";
    // TODO: calculate exact values
    pub const SFT_RECEIVE_MIN_GAS: Gas = Gas::from_tgas(5);
    pub const SFT_RESOLVE_GAS: Gas = Gas::from_tgas(5);

    #[inline]
    pub fn init(owner_id: impl Into<AccountId>, minter_id: impl Into<AccountId>) -> Self {
        Self { minter_id: minter_id.into(), owner_id: owner_id.into(), balance: 0 }
    }

    #[inline]
    pub fn init_state(
        owner_id: impl Into<AccountId>,
        minter_id: impl Into<AccountId>,
    ) -> BTreeMap<Vec<u8>, Vec<u8>> {
        [(
            Self::STATE_KEY.to_vec(),
            borsh::to_vec(&Self::init(owner_id, minter_id)).unwrap_or_else(|_| unreachable!()),
        )]
        .into()
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
}

impl TransferNotification {
    #[inline]
    pub fn msg(msg: String) -> Self {
        Self { state_init: None, msg }
    }

    #[inline]
    pub fn state_init(mut self, state_init: StateInit, amount: NearToken) -> Self {
        self.state_init = Some(StateInitArgs { state_init, state_init_amount: amount });
        self
    }
}

#[near(serializers=[borsh, json])]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StateInitArgs {
    #[serde(flatten)]
    pub state_init: StateInit,

    #[serde(default, skip_serializing_if = "NearToken::is_zero")]
    pub state_init_amount: NearToken,
}

// #[cfg(test)]
// mod tests {
//     use near_sdk::{ContractCode, StateInit};

//     use super::*;

//     #[test]
//     fn storage_cost() {
//         let long_account_id: AccountId = "a".repeat(64).parse().unwrap();

//         let code = ContractCode::GlobalAccountId("wallet.sft.near".parse().unwrap());
//         let data = SFTWalletData::init_state(&long_account_id, &long_account_id);

//         let state_init = StateInit::code(code).data(data);

//         println!("max storage cost: {}", state_init.storage_cost());
//     }
// }
