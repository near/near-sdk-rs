use near_sdk::{ext_contract, json_types::U128, AccountId, ContractCode, PromiseOrValue};

/// # Sharded Fungible Token Minter
///
/// See [Jetton minter](https://docs.ton.org/v3/guidelines/dapps/asset-processing/jettons#jetton-minter).
#[ext_contract(sft_minter)]
pub trait ShardedFungibleTokenMinter {
    /// View-method to calculate [`AccountId`] of wallet-contract for given
    /// `owner_id`, primarily to be used off-chain.
    fn sft_wallet_account_id(&self, owner_id: AccountId) -> AccountId;

    /// Returns total supply of the token
    fn sft_total_supply(&self) -> U128;

    /// Returns contract code used for wallet-contracts (likely to be reference to global)
    fn sft_wallet_code(&self) -> ContractCode;
}

/// Optional "burner" trait for [minter-contract](ShardedFungibleTokenMinter).
#[ext_contract(sft_burner)]
pub trait SharedFungibleTokenBurner: ShardedFungibleTokenMinter {
    /// Performs custom logic for burning tokens.
    /// Returns used amount of tokens successfully burnt, while
    /// `amount - used_amount` (or `amount` if not implemented or panics)
    /// will be minted back on `sender_id`.
    ///
    /// Note: must be `#[payable]` and require at least 1yN attached
    fn sft_on_burn(sender_id: AccountId, amount: U128, msg: String) -> PromiseOrValue<U128>;
}
