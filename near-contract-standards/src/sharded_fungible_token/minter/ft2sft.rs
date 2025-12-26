use std::{
    collections::BTreeMap,
    ops::{Deref, DerefMut},
};

use near_sdk::{borsh, ext_contract, near, AccountId, GlobalContractId, NearToken};

use crate::{
    fungible_token::receiver::FungibleTokenReceiver,
    sharded_fungible_token::{
        minter::{SftMinterData, ShardedFungibleTokenBurner, ShardedFungibleTokenMinter},
        wallet::TransferNotification,
    },
};

/// # Fungible Tokens to Sharded Fungible Tokens adaptor.
///
/// It mints sharded fungible tokens on [`.ft_on_transfer()`](crate::fungible_token::receiver::FungibleTokenReceiver::ft_on_transfer)
/// and burns them back in [`.sft_on_burn()`](crate::sharded_fungible_token::minter::ShardedFungibleTokenBurner::sft_on_burn).
#[ext_contract(ext_ft2sft)]
pub trait Ft2Sft:
    ShardedFungibleTokenMinter + ShardedFungibleTokenBurner + FungibleTokenReceiver
{
    fn ft_contract_id(self) -> AccountId;
}

#[near(serializers = [borsh, json])]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ft2SftData {
    #[serde(flatten)]
    pub data: SftMinterData,

    /// Contract implementing NEP-141 fungible token standard
    pub ft_contract_id: AccountId,
}

/// Message for [`.ft_on_transfer()`](crate::fungible_token::receiver::FungibleTokenReceiver::ft_on_transfer)
#[near(serializers = [borsh, json])]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MintMessage {
    /// Receiver of the sharded FTs, or `sender_id` if not given
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub receiver_id: Option<AccountId>,

    /// Memo to pass in [`.sft_receive()`](crate::sharded_fungible_token::wallet::ShardedFungibleTokenWallet::sft_receive)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub memo: Option<String>,

    /// Optionally, notify `receiver_id` via [`.sft_on_receive()`](crate::sharded_fungible_token::receiver::ShardedFungibleTokenReceiver::sft_on_receive).
    /// Note that non-zero [`forward_deposit`](TransferNotification::forward_deposit)
    /// and [`state_init.state_init_amount`](crate::sharded_fungible_token::wallet::StateInitArgs::state_init_amount)
    /// are not supported, since [`.ft_on_transfer()`](crate::fungible_token::receiver::FungibleTokenReceiver::ft_on_transfer)
    /// doesn't support attaching deposit according to NEP-141 spec.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notify: Option<TransferNotification>,
}

/// Message for [`.sft_on_burn()`](super::ShardedFungibleTokenBurner::sft_on_burn)
#[near(serializers = [borsh, json])]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BurnMessage {
    /// Receiver of the non-sharded FTs, or `sender_id` if not given
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub receiver_id: Option<AccountId>,

    /// Memo to pass in FT transfer.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub memo: Option<String>,

    /// If given, call [`.ft_transfer_call()`](crate::fungible_token::core::FungibleTokenCore::ft_transfer_call)
    /// with given `msg`
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub msg: Option<String>,

    /// If given and non-zero, make [`.storage_deposit()`](crate::storage_management::StorageManagement::storage_deposit)
    /// for receiver before the actual transfer.
    #[serde(default, skip_serializing_if = "NearToken::is_zero")]
    pub storage_deposit: NearToken,

    /// Where to refund excess attached deposit, or `sender_id` if not given.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub refund_to: Option<AccountId>,
}

impl Ft2SftData {
    pub const STATE_KEY: &'static [u8] = b"";

    #[inline]
    pub fn init(
        ft_contract_id: impl Into<AccountId>,
        sft_wallet_code: impl Into<GlobalContractId>,
    ) -> Self {
        Self { data: SftMinterData::init(sft_wallet_code), ft_contract_id: ft_contract_id.into() }
    }

    #[inline]
    pub fn init_state(
        ft_contract_id: impl Into<AccountId>,
        sft_wallet_code: impl Into<GlobalContractId>,
    ) -> BTreeMap<Vec<u8>, Vec<u8>> {
        [(
            Self::STATE_KEY.to_vec(),
            borsh::to_vec(&Self::init(ft_contract_id, sft_wallet_code))
                .unwrap_or_else(|_| unreachable!()),
        )]
        .into()
    }
}

impl Deref for Ft2SftData {
    type Target = SftMinterData;

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl DerefMut for Ft2SftData {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.data
    }
}
