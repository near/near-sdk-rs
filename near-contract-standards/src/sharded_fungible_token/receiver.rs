use near_sdk::{ext_contract, json_types::U128, AccountId, PromiseOrValue};

/// Receiver (i.e. owner_id) of sharded fungible tokens
#[ext_contract(sft_receiver)]
pub trait SharedFungibleTokenReceiver {
    /// Called by wallet-contract upon receiving tokens from `sender_id`
    /// Returns number of used tokens, indicating `amount - used` should be
    /// refunded back to the `sender_id`.
    ///
    /// There are two possible ways to get `minter_id` of just received
    /// tokens:
    /// * Pass it in `msg`, so it can be verified using
    ///   [`near_sdk::StateInit::derived_account_id()`]
    /// * Call view-method `predecessor_account_id::sft_wallet_data()` and
    ///   extract `minter_id`
    /// Note: implementations are recommended to be `#[payable]`.
    fn sft_on_transfer(sender_id: AccountId, amount: U128, msg: String) -> PromiseOrValue<U128>;
}
