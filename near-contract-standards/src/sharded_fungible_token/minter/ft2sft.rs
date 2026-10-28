use near_sdk::{AccountId, near};

use crate::sharded_fungible_token::wallet::TransferNotification;

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
    /// and [`state_init.deposit`](crate::sharded_fungible_token::wallet::StateInitArgs::deposit)
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
}
