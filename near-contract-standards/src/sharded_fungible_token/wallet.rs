use std::borrow::Cow;

use near_sdk::{
    ext_contract, json_types::U128, near, AccountId, AccountIdRef, Gas, NearToken, PromiseOrValue,
    StateInitArgs,
};

/// # Sharded Fungible Token wallet-contract
//
/// The design is highly inspired by [Jetton](https://docs.ton.org/v3/guidelines/dapps/asset-processing/jettons#jetton-architecture)
/// standard except for following differences:
/// * Unlike TVM, Near doesn't support [message bouncing](https://docs.ton.org/v3/documentation/smart-contracts/transaction-fees/forward-fees#message-bouncing),
///   so instead we can schedule callbacks, which gives more control over
///   handling of failed cross-contract calls.
/// * TVM doesn't differentiate between gas and attached deposit, while
///   in Near they are not coupled, which removes some complexities.
///
/// ## Events
///
/// Similar to Jetton standard, there is no logging of such events as
/// `sft_transfer`, `sft_mint` or `sft_burn` as it simply wouldn't bring any
/// value for indexers. Even if we do emit these events, indexers are still
/// forced to track `sft_transfer` function calls to not-yet-existing
/// wallet-contracts, which will emit these events.
///
/// However, to properly track these cross-contract calls they would need
/// parse function names (i.e. `sft_transfer()`, `sft_receive()`, `sft_burn()`
/// and `sft_resolve()`) and their args, while this information combined with
/// receipt status already contains all necessary info for indexing.
#[ext_contract(sft_wallet)]
pub trait ShardedFungibleTokenWallet {
    /// Intialize contract's state.
    ///
    /// Must be annotated with `#[init]`.
    fn init(
        // we use borsh for deterministic serialization
        #[serializer(borsh)] init_args: InitArgs<'static>,
    ) -> ShardedFungibleTokenWalletData;

    /// View method to get all data at once
    fn sft_wallet_data(&self) -> &ShardedFungibleTokenWalletData;

    /// Transfer given `amount` of tokens to `receiver_id`.
    ///
    /// Requires at least [`ShardedFungibleTokenWalletData::MIN_BALANCE`]
    /// attached deposit to reserve for deploying receiver's wallet-contract
    /// if it doesn't exist. If it turned out to be already deployed, then
    /// reserved NEAR tokens are sent to `wallet_init_refund_to`.
    ///
    /// If `notify` is set, then `receiver_id::sft_on_transfer()` will be
    /// called. If `notify.state_init` is set, then `receiver_id` will be
    /// initialized if doesn't exist.
    ///
    /// Remaining attached deposit is forwarded to `receiver_id::sft_on_transfer()`.
    ///
    /// Returns `used_amount`.
    ///
    /// Note: must be #[payable]
    fn sft_transfer(
        &mut self,
        receiver_id: AccountId,
        amount: U128,
        memo: String,
        notify: Option<TransferNotificaton>,
        wallet_init_refund_to: Option<AccountId>,
    ) -> PromiseOrValue<U128>;

    /// Receives tokens from minter-contract or wallet-contracts initialized
    /// for the same minter-contract.
    ///
    /// If `notify` is set, then `receiver_id::sft_on_transfer()` will be
    /// called. If `notify.state_init` is set, then `receiver_id` will be
    /// initialized if doesn't exist.
    ///
    /// Remaining attached deposit is forwarded to `receiver_id::sft_on_transfer()`.
    ///
    /// Returns `used_amount`.
    ///
    /// Note: must be #[payable] and require at least 1yN attached.
    fn sft_receive(
        &mut self,
        sender_id: AccountId,
        amount: U128,
        #[allow(unused_variables)] memo: String,
        notify: Option<TransferNotificaton>,
    ) -> PromiseOrValue<U128>;

    /// Code of this wallet-contract will be re-used across all applications
    /// that want to interact with sharded fungible tokens, so we need a
    /// uniform method to burn tokens to be supported by every wallet-contract.
    /// If the minter-contract doesn't support burning, these tokens
    /// will be minted back on burner wallet-contract.
    ///
    /// Returns `burned_amount`
    ///
    /// Note: must be #[payable] and require at least 1yN attached
    fn sft_burn(&mut self, amount: U128, msg: String) -> PromiseOrValue<U128>;

    /// Gets result from `sft_receive()`, `sft_on_transfer()`
    /// or `sft_on_burn()` and returns `used_amount`.
    ///
    /// Note: must be #[private]
    fn sft_resolve(&mut self, amount: U128, sender: bool) -> U128;
}

/// Sharded Fungible Token wallet-contract data
#[cfg_attr(
    not(feature = "sharded_fungible_token_wallet_impl"),
    near(serializers = [borsh, json]),
)]
#[cfg_attr(
    feature = "sharded_fungible_token_wallet_impl",
    near(contract_state, serializers = [json]),
)]
pub struct ShardedFungibleTokenWalletData {
    pub owner_id: AccountId,
    pub minter_id: AccountId,
    pub balance: U128,
}

impl ShardedFungibleTokenWalletData {
    // TODO: calculate exact values
    pub const MIN_BALANCE: NearToken = NearToken::from_millinear(500);
    pub const SFT_RECEIVE_MIN_GAS: Gas = Gas::from_tgas(5);
    pub const SFT_RESOLVE_GAS: Gas = Gas::from_tgas(5);
}

/// Init args for `.init()`
#[near(serializers = [borsh])]
pub struct InitArgs<'a> {
    pub owner_id: Cow<'a, AccountIdRef>,
    pub minter_id: Cow<'a, AccountIdRef>,
}

/// Arguments for constructing `receiver_id::sft_on_transfer()` notification
#[near(serializers = [borsh, json])]
pub struct TransferNotificaton {
    /// Message be passed in `receiver_id::sft_on_transfer()`
    pub msg: String,
    /// Optionally, deploy & init `receiver_id` contract if didn't exist.
    /// It enables for better composability when transferring to other sharded
    /// contracts, which doesn't exist yet.
    pub state_init: Option<StateInitArgs>,
}
