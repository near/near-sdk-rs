use near_sdk::ext_contract;

use crate::sharded_fungible_token::wallet::ShardedFungibleTokenWallet;

/// # Governed Sharded Fungible Token wallet-contract
///
/// Same as [ShardedFungibleTokenWallet], but
/// [minter-contract](super::minter::ShardedFungibleTokenMinter) is also
/// allowed to:
/// * forcibly transfer by calling [`.sft_transfer()`](ShardedFungibleTokenWallet::sft_transfer)
/// * lock outgoing transfers
/// * lock incoming transfers
#[ext_contract(ext_sft_wallet_governed)]
pub trait ShardedFungibleTokenWalletGoverned: ShardedFungibleTokenWallet {
    /// Set governed status for this owner (only allowed for minter):
    /// * `send`: (un)lock outgoing transfers unless not given
    /// * `receive`: (un)lock incoming transfers unless not given
    ///
    /// Note: MUST have exactly 1yN attached.
    fn sft_governed_set_locked(&mut self, send: Option<bool>, receive: Option<bool>);

    /// Returns whether outgoing transfers are locked for this owner.
    fn sft_governed_is_send_locked(&self) -> bool;
    /// Returns whether incoming transfers are locked for this owner.
    fn sft_governed_is_receive_locked(&self) -> bool;
}
