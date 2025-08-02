pub mod ft2sft;

use near_sdk::{
    ext_contract, json_types::U128, near, serde_with::DisplayFromStr, AccountId, ContractCode,
    PromiseOrValue,
};

use crate::contract_state::ContractState;

/// # Sharded Fungible Token Minter
///
/// This is a contracts that is allowed to mint new tokens to and burn them from
/// [wallet-contracts](super::wallet::ShardedFungibleTokenWallet).
///
/// See [Jetton minter](https://docs.ton.org/v3/guidelines/dapps/asset-processing/jettons#jetton-minter).
#[ext_contract(ext_sft_minter)]
pub trait ShardedFungibleTokenMinter {
    /// View method to get all data at once
    fn sft_minter_data(self) -> ContractState<SftMinterData>;

    /// View-method to calculate [`AccountId`] of wallet-contract for given
    /// `owner_id`, primarily to be used off-chain.
    fn sft_wallet_account_id(&self, owner_id: AccountId) -> AccountId;
}

/// Common data for all [minter-contract](ShardedFungibleTokenMinter)
/// implementations.
#[near(serializers = [borsh, json])]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SftMinterData {
    /// Total amount of fungible tokens minted
    #[serde_as(as = "DisplayFromStr")]
    pub total_supply: u128,

    /// Code for deploying child wallet-contracts
    pub sft_wallet_code: ContractCode,
}

impl SftMinterData {
    #[inline]
    pub fn init(sft_wallet_code: impl Into<ContractCode>) -> Self {
        Self { total_supply: 0, sft_wallet_code: sft_wallet_code.into() }
    }
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
