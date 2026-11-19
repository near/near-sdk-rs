#[cfg(feature = "ft2sft")]
mod ft2sft;

use core::ops::{Deref, DerefMut};

use near_contract_standards::sharded_fungible_token::{
    minter::{
        GovernedSftMinterData, ShardedFungibleTokenMinter, ShardedFungibleTokenMinterGoverned,
    },
    wallet::{SftWalletData, governed::ext_sft_wallet_governed},
};
use near_sdk::{
    AccountId, GlobalContractId, NearToken, PanicOnDefault, Promise, env,
    json_types::U128,
    near, require,
    state_init::{StateInit, StateInitV1},
};

/// Reference implementation for
/// [Fungible Tokens to Sharded Fungible Tokens adaptor](Ft2Sft)
#[near(contract_state(key = GovernedSftMinterData::STATE_KEY))]
#[derive(PanicOnDefault)]
#[repr(transparent)]
struct Contract(GovernedSftMinterData);

#[near]
/// Minter is governed by default, while it depends on child wallet-contract
/// code if it implements governed functionality or not
impl ShardedFungibleTokenMinterGoverned for Contract {
    #[payable]
    fn sft_minter_transfer_authority(&mut self, new_authority_id: AccountId) {
        require!(
            env::attached_deposit() == NearToken::from_yoctonear(1),
            Self::ERR_INSUFFICIENT_DEPOSIT,
        );
        require!(env::predecessor_account_id() == *self.authority_id, Self::ERR_UNAUTHORIZED);

        self.authority_id = new_authority_id;
    }

    /// If the sFT wallet-contract code has governance capabilities, then
    /// FT contract can set locked status for specific owner.
    ///
    /// Note: MUST have exactly 1yN attached.
    #[payable]
    fn sft_minter_set_locked_for(
        &mut self,
        owner_id: AccountId,
        send: Option<bool>,
        receive: Option<bool>,
    ) -> Promise {
        require!(
            env::attached_deposit() == NearToken::from_yoctonear(1),
            Self::ERR_INSUFFICIENT_DEPOSIT,
        );
        require!(env::predecessor_account_id() == *self.authority_id, Self::ERR_UNAUTHORIZED);

        ext_sft_wallet_governed::ext_on({
            let state_init = self.sft_wallet_init_for(owner_id);
            Promise::new(state_init.derive_account_id())
                // init owner's wallet-contract as it might be not
                // initialized yet
                .state_init(
                    state_init,
                    // sFT wallet-contract fits into ZBA limits, i.e. < 770 bytes
                    NearToken::ZERO,
                )
        })
        .with_attached_deposit(NearToken::from_yoctonear(1))
        .sft_governed_set_locked(send, receive)
    }

    /// Fungible Token contract id
    fn sft_minter_authority_id(&self) -> AccountId {
        self.authority_id.clone()
    }
}

#[near]
impl ShardedFungibleTokenMinter for Contract {
    fn sft_total_supply(&self) -> U128 {
        self.total_supply.into()
    }

    fn sft_wallet_global_contract_id(&self) -> GlobalContractId {
        self.sft_wallet_code.clone()
    }

    fn sft_wallet_account_id_for(&self, owner_id: AccountId) -> AccountId {
        self.sft_wallet_account_id_for(&owner_id)
    }
}

impl Contract {
    const ERR_UNAUTHORIZED: &str = "unauthorized";
    const ERR_INSUFFICIENT_DEPOSIT: &str = "insufficient attached deposit";

    fn sft_wallet_init_for(&self, owner_id: impl Into<AccountId>) -> StateInit {
        StateInit::V1(StateInitV1 {
            code: self.sft_wallet_code.clone(),
            data: SftWalletData::init_state(owner_id, env::current_account_id()),
        })
    }

    fn sft_wallet_account_id_for(&self, owner_id: impl Into<AccountId>) -> AccountId {
        self.sft_wallet_init_for(owner_id).derive_account_id()
    }
}

impl Deref for Contract {
    type Target = GovernedSftMinterData;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Contract {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
