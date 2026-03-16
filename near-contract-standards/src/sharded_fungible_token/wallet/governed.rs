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
    /// Lock outgoing transfers for this owner (only allowed for minter).
    ///
    /// Note: MUST have exactly 1yN attached.
    fn sft_wallet_lock_send(&mut self);

    /// Lock incoming transfers for this owner (only allowed for minter).
    ///
    /// Note: MUST have exactly 1yN attached.
    fn sft_wallet_lock_receive(&mut self);
}
