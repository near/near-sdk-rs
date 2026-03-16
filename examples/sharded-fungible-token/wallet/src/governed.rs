use bitflags::bitflags;
use near_contract_standards::sharded_fungible_token::wallet::governed::ShardedFungibleTokenWalletGoverned;
use near_sdk::{NearToken, env, near, require};

use crate::{SftWalletContract, SftWalletContractExt};

#[near]
impl ShardedFungibleTokenWalletGoverned for SftWalletContract {
    /// Set governed status for this owner (only allowed for minter):
    /// * `send`: (un)lock outgoing transfers unless not given
    /// * `receive`: (un)lock incoming transfers unless not given
    ///
    /// Note: MUST have exactly 1yN attached.
    fn sft_governed_set_locked(&mut self, send: Option<bool>, receive: Option<bool>) {
        require!(
            env::attached_deposit() == NearToken::from_yoctonear(1),
            Self::ERR_INSUFFICIENT_DEPOSIT,
        );
        require!(env::predecessor_account_id() == *self.minter_id, Self::ERR_NOT_OWNER);

        let mut status = self.status();

        if let Some(lock) = send {
            status.set(Status::SEND_LOCKED, lock);
        }
        if let Some(lock) = receive {
            status.set(Status::RECEIVE_LOCKED, lock);
        }

        self.set_status(status);
    }
    /// Returns whether outgoing transfers are locked for this owner.
    fn sft_governed_is_send_locked(&self) -> bool {
        self.status().contains(Status::SEND_LOCKED)
    }
    /// Returns whether incoming transfers are locked for this owner.
    fn sft_governed_is_receive_locked(&self) -> bool {
        self.status().contains(Status::RECEIVE_LOCKED)
    }
}

bitflags! {
    #[derive(Clone, Copy)]
    struct Status: u8 {
        const SEND_LOCKED    = 1 << 0;
        const RECEIVE_LOCKED = 1 << 1;
    }
}

impl SftWalletContract {
    pub const ERR_LOCKED: &str = "wallet is locked";

    const STATUS_KEY: &[u8] = b"s";

    #[allow(clippy::unused_self)]
    fn status(&self) -> Status {
        env::storage_read(Self::STATUS_KEY)
            .and_then(|value| {
                let [byte] = value.try_into().ok()?;
                Some(byte)
            })
            .map_or_else(Status::empty, Status::from_bits_retain)
    }

    #[allow(clippy::unused_self, clippy::needless_pass_by_ref_mut)]
    fn set_status(&mut self, status: Status) {
        env::storage_write(Self::STATUS_KEY, &[status.bits()]);
    }
}
