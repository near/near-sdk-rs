use near_sdk::{ext_contract, json_types::U128, AccountId, PromiseOrValue};

/// Receiver (i.e. owner_id) of sharded fungible tokens
#[ext_contract(ext_sft_receiver)]
pub trait ShardedFungibleTokenReceiver {
    /// Called by wallet-contract upon receiving tokens.
    ///
    /// Returns number of used tokens, meaning that `amount - used` should be
    /// refunded back to the `sender_id`.
    ///
    /// Note: `amount` can be zero.
    ///
    /// There are two possible ways to get `minter_id` of just received
    /// tokens:
    /// * Pass it in `msg`, so it can be verified using
    ///   [`::near_sdk::StateInit::derive_account_id()`]
    /// * Call view-method [`predecessor_account_id::sft_wallet_data()`](super::wallet::ShardedFungibleTokenWallet::sft_wallet_data)
    ///   and extract `data.minter_id`
    ///
    /// WARN: DO NOT BLINDLY TRUST `sender_id`, malicious minter can propagate
    /// arbitrary sender in [`.sft_receive()`](super::wallet::ShardedFungibleTokenWallet::sft_receive).
    ///
    /// Note: MUST be `#[payable]` and require at least 1yN attached.
    fn sft_on_receive(
        &mut self,
        sender_id: AccountId,
        amount: U128,
        msg: String,
    ) -> PromiseOrValue<U128>;
}
