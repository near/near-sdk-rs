use impl_tools::autoimpl;
use near_contract_standards::sharded_fungible_token::{
    minter::ext_sft_burner,
    receiver::ext_sft_receiver,
    wallet::{
        SftWalletData, ShardedFungibleTokenWallet, StateInitArgs, TransferNotification,
        events::{SftEvent, SftReceive, SftSend},
    },
};
use near_sdk::{
    AccountId, AccountIdRef, NearToken, PanicOnDefault, Promise, PromiseOrValue, env,
    json_types::U128,
    near, require, serde_json,
    state_init::{StateInit, StateInitV1},
};

/// # Reference implementation of Sharded Fungible Foken
/// [wallet-contract](ShardedFungibleTokenWallet).
///
/// This implementation should be globally deployed only once and can
/// then be reused for different minter-contracts. Owners might reference
/// its globally deployed code to calculate [`StateInit`] and verify
/// authenticity of [`env::predecessor_account_id()`] via
/// [`.derive_account_id()`](StateInit::derive_account_id).
///
/// The implementation is highly inspired by [Jetton wallet](https://github.com/ton-blockchain/jetton-contract/blob/3d24b419f2ce49c09abf6b8703998187fe358ec9/contracts/jetton-wallet.fc)
/// contract reference implementation.
#[near(contract_state(key = SftWalletData::STATE_KEY))]
#[autoimpl(Deref using self.0)]
#[autoimpl(DerefMut using self.0)]
#[derive(PanicOnDefault)]
#[repr(transparent)]
struct SFTWalletContract(SftWalletData);

#[near]
impl ShardedFungibleTokenWallet for SFTWalletContract {
    /// View method to get all data at once
    fn sft_wallet_data(self) -> SftWalletData {
        self.0
    }

    /// Transfer given `amount` of tokens to `receiver_id`.
    ///
    /// Unless `no_init` is set, requires additional attached deposit to pay for
    /// automatic deployment and initialization of receiver's wallet-contract.
    /// If by the time the created receipt gets executed it turns out to be
    /// already deployed, then reserved NEAR tokens refunded to `refund_to` or
    /// predecessor, if not set. See [`near_sdk::StateInit`] and
    /// [`near_sdk::env::promise_set_refund_to()`].
    ///
    /// If `notify` is set, then [`receiver_id::sft_on_transfer()`](super::receiver::ShardedFungibleTokenReceiver)
    /// will be called. If `notify.state_init` is set, then `receiver_id` will
    /// be initialized if doesn't exist. See [`TransferNotification`].
    ///
    /// Returns `used_amount`.
    #[payable]
    fn sft_send(
        &mut self,
        receiver_id: AccountId,
        amount: U128,
        memo: Option<String>,
        notify: Option<TransferNotification>,
    ) -> PromiseOrValue<U128> {
        // require at least 1yN attached for security measures
        require!(
            env::attached_deposit() >= NearToken::from_yoctonear(1),
            Self::ERR_INSUFFICIENT_DEPOSIT,
        );
        require!(amount.0 > 0, Self::ERR_ZERO_AMOUNT);

        let caller = env::predecessor_account_id();
        #[cfg(not(feature = "governed"))]
        require!(caller == *self.owner_id, Self::ERR_NOT_OWNER);
        #[cfg(feature = "governed")]
        if caller != *self.minter_id {
            require!(caller == *self.owner_id, Self::ERR_NOT_OWNER);
            require!(!self.is_outgoing_transfers_locked(), Self::ERR_LOCKED);
        }

        require!(receiver_id != *self.owner_id, Self::ERR_SELF_TRANSFER);

        self.balance = self
            .balance
            .checked_sub(amount.0)
            .unwrap_or_else(|| env::panic_str(Self::ERR_INSUFFICIENT_BALANCE));

        SftEvent::Send(
            [SftSend {
                receiver_id: (&receiver_id).into(),
                amount: amount.0,
                memo: memo.as_deref().map(Into::into),
            }]
            .as_slice()
            .into(),
        )
        .emit();

        Self::ext_on({
            let state_init = self.sft_wallet_init_for(&receiver_id);
            Promise::new(state_init.derive_account_id())
                // always deploy & init receiver's wallet-contract
                .state_init(
                    state_init,
                    // sFT wallet-contract fits into ZBA limits, i.e. < 770 bytes
                    NearToken::ZERO,
                )
                // refund attached deposit in case of failure to `refund_to` set for current receipt
                // (or predecessor, otherwise) instead of sender's wallet-contract
                .refund_to(env::refund_to_account_id())
        })
        // forward attached deposit (at least 1yN)
        .with_attached_deposit(env::attached_deposit())
        // require minimum gas
        .with_static_gas(SftWalletData::SFT_RECEIVE_MIN_GAS)
        // forward all remaining gas here
        .with_unused_gas_weight(1)
        .sft_receive(self.owner_id.clone(), amount, memo, notify)
        .then(
            Self::ext(env::current_account_id())
                .with_static_gas(SftWalletData::SFT_RESOLVE_GAS)
                // do not distribute remaining gas here
                .with_unused_gas_weight(0)
                .sft_resolve_transfer(amount, true, receiver_id),
        )
        .into()
    }

    /// Receives tokens from [minter-contract](super::minter::ShardedFungibleTokenMinter)
    /// or wallet-contracts initialized for the same minter-contract.
    ///
    /// If `notify` is set, then `receiver_id::sft_on_transfer()` will be
    /// called. If `notify.state_init` is set, then `receiver_id` will be
    /// initialized if doesn't exist.
    ///
    /// Remaining attached deposit is refunded to `refund_to` or `sender_id`
    /// if not set.
    ///
    /// Returns `used_amount`.
    #[payable]
    fn sft_receive(
        &mut self,
        sender_id: AccountId,
        amount: U128,
        memo: Option<String>,
        notify: Option<TransferNotification>,
    ) -> PromiseOrValue<U128> {
        require!(amount.0 > 0, Self::ERR_ZERO_AMOUNT);

        let caller = env::predecessor_account_id();
        // verify that the caller is the minter or a valid wallet-contract
        require!(
            caller == *self.minter_id || caller == self.sft_wallet_account_id_for(&sender_id),
            Self::ERR_WRONG_WALLET,
        );
        #[cfg(feature = "governed")]
        require!(!self.is_incoming_transfers_locked(), Self::ERR_LOCKED);

        self.balance = self
            .balance
            .checked_add(amount.0)
            .unwrap_or_else(|| env::panic_str(Self::ERR_BALANCE_OVERFLOW));

        SftEvent::Receive(
            [SftReceive {
                sender_id: (&sender_id).into(),
                amount: amount.0,
                memo: memo.as_deref().map(Into::into),
            }]
            .as_slice()
            .into(),
        )
        .emit();

        let Some(notify) = notify else {
            // no transfer notification, all tokens received
            return PromiseOrValue::Value(amount);
        };

        let mut deposit_left = env::attached_deposit();

        // notify receiver
        let mut p = Promise::new(self.owner_id.clone())
            // refund state_init amount and attached deposit in case of failure
            // to `refund_to` set by sender (or sender, otherwise) instead of
            // receiver's wallet-contract
            .refund_to(env::refund_to_account_id());

        if let Some(StateInitArgs { state_init, state_init_amount }) = notify.state_init {
            deposit_left = deposit_left
                .checked_sub(state_init_amount)
                .unwrap_or_else(|| env::panic_str(Self::ERR_INSUFFICIENT_DEPOSIT));

            p = p.state_init(state_init, state_init_amount);
        }

        require!(deposit_left >= NearToken::from_yoctonear(1), Self::ERR_INSUFFICIENT_DEPOSIT);

        ext_sft_receiver::ext_on(p)
            // forward deposit
            .with_attached_deposit(deposit_left)
            // forward all remaining gas here
            .with_unused_gas_weight(1)
            .sft_on_receive(sender_id.clone(), amount, notify.msg)
            .then(
                Self::ext(env::current_account_id())
                    .with_static_gas(SftWalletData::SFT_RESOLVE_GAS)
                    // do not distribute remaining gas here
                    .with_unused_gas_weight(0)
                    // resolve notification
                    .sft_resolve_transfer(amount, false, sender_id),
            )
            .into()
    }

    /// Burn given `amount` and notify [`minter_id::sft_on_burn()`](super::minter::ShardedFungibleTokenBurner::sft_on_burn).
    /// If `minter_id` doesn't support burning or returns partial
    /// `used_amount`, then `amount - used_amount` will be minter back
    /// on `sender_id`.
    ///
    /// Code of this wallet-contract will be re-used across all applications
    /// that want to interact with sharded fungible tokens, so we need a
    /// uniform method to burn tokens to be supported by every wallet-contract.
    /// If the minter-contract doesn't support burning, these tokens
    /// will be minted back on burner's wallet-contract.
    ///
    /// Returns `burned_amount`.
    #[payable]
    fn sft_burn(
        &mut self,
        amount: U128,
        memo: Option<String>,
        msg: String,
    ) -> PromiseOrValue<U128> {
        require!(
            env::attached_deposit() >= NearToken::from_yoctonear(1),
            Self::ERR_INSUFFICIENT_DEPOSIT
        );
        require!(amount.0 > 0, Self::ERR_ZERO_AMOUNT);

        let caller = env::predecessor_account_id();

        #[cfg(not(feature = "governed"))]
        require!(caller == *self.owner_id, Self::ERR_NOT_OWNER);
        #[cfg(feature = "governed")]
        if caller != *self.minter_id {
            require!(caller == *self.owner_id, Self::ERR_NOT_OWNER);
            require!(!self.is_outgoing_transfers_locked(), Self::ERR_LOCKED);
        }

        self.balance = self
            .balance
            .checked_sub(amount.0)
            .unwrap_or_else(|| env::panic_str(Self::ERR_INSUFFICIENT_BALANCE));

        SftEvent::Send(
            [SftSend {
                receiver_id: (&self.minter_id).into(),
                amount: amount.0,
                memo: memo.as_deref().map(Into::into),
            }]
            .as_slice()
            .into(),
        )
        .emit();

        ext_sft_burner::ext(self.minter_id.clone())
            // forward all attached deposit
            .with_attached_deposit(env::attached_deposit())
            // forward all remaining gas here
            .with_unused_gas_weight(1)
            .sft_on_burn(self.owner_id.clone(), amount, msg)
            .then(
                Self::ext(env::current_account_id())
                    .with_static_gas(SftWalletData::SFT_RESOLVE_GAS)
                    // do not distribute remaining gas here
                    .with_unused_gas_weight(0)
                    .sft_resolve_transfer(amount, true, self.minter_id.clone()),
            )
            .into()
    }
}

#[near]
impl SFTWalletContract {
    /// Gets result from `sft_receive()`, `sft_on_receive()`
    /// or `sft_on_burn()`, adjusts the balance accordingly
    /// and returns `used_amount`.
    #[allow(dead_code)]
    #[private]
    pub fn sft_resolve_transfer(
        &mut self,
        amount: U128,
        sender: bool,
        account_id: AccountId,
    ) -> U128 {
        const MAX_RESULT_LENGTH: usize = "\"+340282366920938463463374607431768211455\"".len(); // u128::MAX
        // TODO: promise_result_at_most
        let mut used_amount = env::promise_result_checked(
            0,
            MAX_RESULT_LENGTH, // prevent out of gas (too long result)
        )
        .ok() // promise failed or result was too long
        .and_then(|data| serde_json::from_slice::<U128>(&data).ok()) // JSON
        .unwrap_or_default() // if any of above failed, then refund full amount
        .0
        .min(amount.0); // do not refund more than we sent

        let mut refund_amount = amount.0.saturating_sub(used_amount);

        self.balance = if sender {
            // add refund_amount to sender, but in checked way:
            // faulty minter-contract implementation could have minted
            // too many tokens after `.sft_resolve()` was scheduled
            // but not executed yet
            self.balance
                .checked_add(refund_amount)
                // this is the only place where we do panic but it's ok,
                // since it can only happen because of the faulty minter
                .unwrap_or_else(|| env::panic_str(Self::ERR_BALANCE_OVERFLOW))
        } else {
            // refund maximum what we can
            refund_amount = refund_amount.min(self.balance);
            // update used_amount
            used_amount = amount.0.saturating_sub(refund_amount);
            // subtract refund from receiver
            self.balance.saturating_sub(refund_amount)
        };

        if refund_amount > 0 {
            if sender {
                SftEvent::Receive(
                    [SftReceive {
                        sender_id: account_id.into(),
                        amount: refund_amount,
                        memo: Some("refund".into()),
                    }]
                    .as_slice()
                    .into(),
                )
                .emit();
            } else {
                SftEvent::Send(
                    [SftSend {
                        receiver_id: account_id.into(),
                        amount: refund_amount,
                        memo: Some("refund".into()),
                    }]
                    .as_slice()
                    .into(),
                )
                .emit();
            }
        }

        U128(used_amount)
    }
}

#[near(serializers = [json])]
#[serde(tag = "op", rename_all = "snake_case")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Op {
    SftSend { receiver_id: AccountId },
    SftReceive { sender_id: AccountId },
    SftBurn,
}

impl Op {
    #[inline]
    pub fn is_sender(&self) -> bool {
        matches!(self, Self::SftSend { .. } | Self::SftBurn)
    }
}

impl SFTWalletContract {
    const ERR_NOT_OWNER: &str = "not owner";
    const ERR_SELF_TRANSFER: &str = "self-transfer";
    const ERR_WRONG_WALLET: &str = "wrong wallet";
    const ERR_ZERO_AMOUNT: &str = "zero amount";
    const ERR_INSUFFICIENT_BALANCE: &str = "insufficient balance";
    const ERR_BALANCE_OVERFLOW: &str = "balance overflow";
    const ERR_INSUFFICIENT_DEPOSIT: &str = "insufficient attached deposit";

    #[inline]
    pub fn sft_wallet_init_for(&self, owner_id: &AccountIdRef) -> StateInit {
        StateInit::V1(StateInitV1 {
            code: env::current_global_contract_id().expect("globally deployed"),
            data: SftWalletData::init_state(owner_id, &*self.minter_id),
        })
    }

    #[inline]
    pub fn sft_wallet_account_id_for(&self, owner_id: &AccountIdRef) -> AccountId {
        self.sft_wallet_init_for(owner_id).derive_account_id()
    }
}

#[cfg(feature = "governed")]
const _: () = {
    use near_contract_standards::sharded_fungible_token::wallet::ShardedFungibleTokenWalletGoverned;

    #[near]
    impl ShardedFungibleTokenWalletGoverned for SFTWalletContract {
        /// Set status (only allowed for minter).
        ///
        /// Note: MUST have exactly 1yN attached.
        #[payable]
        fn sft_wallet_set_status(&mut self, status: u8) {
            require!(
                env::attached_deposit() == NearToken::from_yoctonear(1),
                Self::ERR_INSUFFICIENT_DEPOSIT
            );
            require!(env::predecessor_account_id() == *self.minter_id, Self::ERR_WRONG_WALLET);
            self.status = status;
        }
    }

    impl SFTWalletContract {
        const ERR_LOCKED: &str = "wallet is locked";

        const OUTGOING_TRANSFERS_LOCKED_FLAG: u8 = 1 << 0;
        const INCOMING_TRANSFERS_LOCKED_FLAG: u8 = 1 << 1;

        pub const fn is_outgoing_transfers_locked(&self) -> bool {
            self.0.status & Self::OUTGOING_TRANSFERS_LOCKED_FLAG
                == Self::OUTGOING_TRANSFERS_LOCKED_FLAG
        }

        pub const fn is_incoming_transfers_locked(&self) -> bool {
            self.0.status & Self::INCOMING_TRANSFERS_LOCKED_FLAG
                == Self::INCOMING_TRANSFERS_LOCKED_FLAG
        }
    }
};
