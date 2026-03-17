pub mod ft2sft;

use std::collections::BTreeMap;

use near_sdk::{
    AccountId, GlobalContractId, Promise, PromiseOrValue, borsh, ext_contract, json_types::U128,
    near,
};

/// # Sharded Fungible Token Minter
///
/// This is a contracts that is allowed to mint new tokens to
/// [wallet-contracts](super::wallet::ShardedFungibleTokenWallet).
///
/// See [Jetton minter](https://docs.ton.org/v3/guidelines/dapps/asset-processing/jettons#jetton-minter).
#[ext_contract(ext_sft_minter)]
pub trait ShardedFungibleTokenMinter {
    /// View-method to get the total supply
    fn sft_total_supply(&self) -> U128;

    /// View-method to get global contract id for deploying child
    /// wallet-contracts
    fn sft_wallet_global_contract_id(&self) -> GlobalContractId;

    /// Helper view-method to calculate [`AccountId`] of wallet-contract for
    /// given `owner_id`, primarily to be used off-chain.
    fn sft_wallet_account_id_for(&self, owner_id: AccountId) -> AccountId;
}

/// Optional "burner" trait for [minter-contract](ShardedFungibleTokenMinter).
#[ext_contract(ext_sft_burner)]
pub trait ShardedFungibleTokenBurner: ShardedFungibleTokenMinter {
    /// Performs custom logic for burning tokens.
    /// Returns used amount of tokens successfully burnt, while
    /// `amount - used_amount` (or `amount` if not implemented or panics)
    /// will be minted back on `sender_id`.
    ///
    /// Note: must be `#[payable]` and require at least 1yN attached
    fn sft_on_burn(
        &mut self,
        sender_id: AccountId,
        amount: U128,
        msg: String,
    ) -> PromiseOrValue<U128>;
}

/// # Governed Sharded Fungible Tokens [minter](ShardedFungibleTokenMinter).
///
/// Allows a single `sft_minter_owner_id` to lock incoming/outcoming
/// transfers on child [wallet-contracts](super::wallet::ShardedFungibleTokenWallet).
#[ext_contract(ext_sft_minter_governed)]
pub trait ShardedFungibleTokenMinterGoverned: ShardedFungibleTokenMinter {
    /// Transfer ownership to a new authority.
    ///
    /// Note: must be `#[payable]` and require at least 1yN attached
    fn sft_minter_transfer_authority(&mut self, new_authority_id: AccountId);

    /// If the sFT wallet-contract code has governance capabilities, then
    /// FT contract can set locked status for specific owner.
    ///
    /// Allowed only for `sft_minter_owner_id`
    ///
    /// NOTE: requires 1yN attached deposit.
    fn sft_minter_set_locked_for(
        &mut self,
        owner_id: AccountId,
        send: Option<bool>,
        receive: Option<bool>,
    ) -> Promise;

    /// Owner authority
    fn sft_minter_authority_id(&self) -> AccountId;
}

#[near(serializers = [borsh])]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GovernedSftMinterData {
    /// Contract implementing NEP-141 fungible token standard
    pub authority_id: AccountId,

    /// Code for deploying child wallet-contracts
    pub sft_wallet_code: GlobalContractId,

    /// Total amount of fungible tokens minted
    pub total_supply: u128,
}

impl GovernedSftMinterData {
    pub const STATE_KEY: &'static [u8] = b"";

    #[inline]
    pub fn init(
        authority_id: impl Into<AccountId>,
        sft_wallet_code: impl Into<GlobalContractId>,
    ) -> Self {
        Self {
            authority_id: authority_id.into(),
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
