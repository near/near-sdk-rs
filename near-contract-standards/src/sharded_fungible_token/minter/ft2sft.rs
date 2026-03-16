use std::collections::BTreeMap;

use near_sdk::{AccountId, GlobalContractId, Promise, borsh, ext_contract, near};

use crate::{
    fungible_token::receiver::FungibleTokenReceiver,
    sharded_fungible_token::{
        minter::{ShardedFungibleTokenBurner, ShardedFungibleTokenMinter},
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
    /// Fungible Token contract id
    fn ft_contract_id(&self) -> AccountId;

    /// If the sFT wallet-contract code has governance capabilities, then
    /// FT contract can set locked status for specific owner.
    /// 
    /// NOTE: requires 1yN attached deposit.
    fn sft_governed_set_locked_for(
        &mut self,
        owner_id: AccountId,
        send: Option<bool>,
        receive: Option<bool>,
    ) -> Promise;
}

#[near(serializers = [borsh])]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ft2SftData {
    /// Contract implementing NEP-141 fungible token standard
    pub ft_contract_id: AccountId,

    /// Code for deploying child wallet-contracts
    pub sft_wallet_code: GlobalContractId,

    /// Total amount of fungible tokens minted
    pub total_supply: u128,
    // TODO: feature nep245 + token_id
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

impl Ft2SftData {
    pub const STATE_KEY: &'static [u8] = b"";

    #[inline]
    pub fn init(
        ft_contract_id: impl Into<AccountId>,
        sft_wallet_code: impl Into<GlobalContractId>,
    ) -> Self {
        Self {
            ft_contract_id: ft_contract_id.into(),
            sft_wallet_code: sft_wallet_code.into(),
            total_supply: 0,
        }
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
