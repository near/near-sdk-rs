use near_sdk::ext_contract;

use crate::sharded_fungible_token::wallet::ShardedFungibleTokenWallet;

/// # Governed Sharded Fungible Token wallet-contract
///
/// Same as [ShardedFungibleTokenWallet], but
/// [minter-contract](super::minter::ShardedFungibleTokenMinter) is also
/// allowed to:
/// * forcily transfer by calling [`.sft_transfer()`](ShardedFungibleTokenWallet::sft_transfer)
/// * lock outgoing transfers
/// * lock incoming transfers
#[ext_contract(ext_sft_wallet_governed)]
pub trait ShardedFungibleTokenWalletGoverned: ShardedFungibleTokenWallet {
    /// Set status (only allowed for minter).
    ///
    /// Note: MUST have exactly 1yN attached.
    fn sft_wallet_set_status(&mut self, status: u8);
}
