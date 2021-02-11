use crate::*;
use near_sdk::json_types::ValidAccountId;
use near_sdk::{ext_contract, Gas, PromiseResult};

const GAS_FOR_RESOLVE_TRANSFER: Gas = 5_000_000_000_000;
const GAS_FOR_FT_TRANSFER_CALL: Gas = 25_000_000_000_000 + GAS_FOR_RESOLVE_TRANSFER;

const NO_DEPOSIT: Balance = 0;

pub trait FungibleTokenCore {
    /// Transfers positive `amount` of tokens from the `env::predecessor_account_id` to `receiver_id`.
    /// Both accounts must be registered with the contract for transfer to succeed. (See [NEP-145](https://github.com/near/NEPs/discussions/145))
    /// This method must to be able to accept attached deposits, and must not panic on attached deposit.
    /// Exactly 1 yoctoNEAR must be attached.
    /// See [the Security section](https://github.com/near/NEPs/issues/141#user-content-security) of the standard.
    ///
    /// Arguments:
    /// - `receiver_id` - the account ID of the receiver.
    /// - `amount` - the amount of tokens to transfer. Must be a positive number in decimal string representation.
    /// - `memo` - an optional string field in a free form to associate a memo with this transfer.
    fn ft_transfer(&mut self, receiver_id: ValidAccountId, amount: U128, memo: Option<String>);

    /// Transfers positive `amount` of tokens from the `env::predecessor_account_id` to `receiver_id` account. Then
    /// calls `ft_on_transfer` method on `receiver_id` contract and attaches a callback to resolve this transfer.
    /// `ft_on_transfer` method must return the amount of tokens unused by the receiver contract, the remaining tokens
    /// must be refunded to the `predecessor_account_id` at the resolve transfer callback.
    ///
    /// Token contract must pass all the remaining unused gas to the `ft_on_transfer` call.
    ///
    /// Malicious or invalid behavior by the receiver's contract:
    /// - If the receiver contract promise fails or returns invalid value, the full transfer amount must be refunded.
    /// - If the receiver contract overspent the tokens, and the `receiver_id` balance is lower than the required refund
    /// amount, the remaining balance must be refunded. See [the Security section](https://github.com/near/NEPs/issues/141#user-content-security) of the standard.
    ///
    /// Both accounts must be registered with the contract for transfer to succeed. (See #145)
    /// This method must to be able to accept attached deposits, and must not panic on attached deposit. Exactly 1 yoctoNEAR must be attached. See [the Security
    /// section](https://github.com/near/NEPs/issues/141#user-content-security) of the standard.
    ///
    /// Arguments:
    /// - `receiver_id` - the account ID of the receiver contract. This contract will be called.
    /// - `amount` - the amount of tokens to transfer. Must be a positive number in a decimal string representation.
    /// - `msg` - a string message that will be passed to `ft_on_transfer` contract call.
    /// - `memo` - an optional string field in a free form to associate a memo with this transfer.
    ///
    /// Returns a promise which will result in the amount of tokens withdrawn from sender's account.
    fn ft_transfer_call(
        &mut self,
        receiver_id: ValidAccountId,
        amount: U128,
        msg: String,
        memo: Option<String>,
    ) -> Promise;

    /// Returns the total supply of the token in a decimal string representation.
    fn ft_total_supply(&self) -> U128;

    /// Returns the balance of the account. If the account doesn't exist must returns `"0"`.
    fn ft_balance_of(&self, account_id: ValidAccountId) -> U128;
}

#[ext_contract(ext_fungible_token_receiver)]
trait FungibleTokenReceiver {
    /// Called by fungible token contract after `ft_transfer_call` was initiated by
    /// `sender_id` of the given `amount` with the transfer message given in `msg` field.
    /// The `amount` of tokens were already transferred to this contract account and ready to be used.
    ///
    /// The method must return the amount of tokens that are *not* used/accepted by this contract from the transferred
    /// amount. Examples:
    /// - The transferred amount was `500`, the contract completely takes it and must return `0`.
    /// - The transferred amount was `500`, but this transfer call only needs `450` for the action passed in the `msg`
    ///   field, then the method must return `50`.
    /// - The transferred amount was `500`, but the action in `msg` field has expired and the transfer must be
    ///   cancelled. The method must return `500` or panic.
    ///
    /// Arguments:
    /// - `sender_id` - the account ID that initiated the transfer.
    /// - `amount` - the amount of tokens that were transferred to this account in a decimal string representation.
    /// - `msg` - a string message that was passed with this transfer call.
    ///
    /// Returns the amount of unused tokens that should be returned to sender, in a decimal string representation.
    fn ft_on_transfer(&mut self, sender_id: AccountId, amount: U128, msg: String) -> Promise;
}

#[ext_contract(ext_self)]
trait FungibleTokenResolver {
    fn ft_resolve_transfer(
        &mut self,
        sender_id: AccountId,
        receiver_id: AccountId,
        amount: U128,
    ) -> U128;
}

trait FungibleTokenResolver {
    fn ft_resolve_transfer(
        &mut self,
        sender_id: AccountId,
        receiver_id: AccountId,
        amount: U128,
    ) -> U128;
}

#[near_bindgen]
impl FungibleTokenCore for Contract {
    #[payable]
    fn ft_transfer(&mut self, receiver_id: ValidAccountId, amount: U128, memo: Option<String>) {
        assert_one_yocto();
        let sender_id = env::predecessor_account_id();
        let amount = amount.into();
        self.internal_transfer(&sender_id, receiver_id.as_ref(), amount, memo);
    }

    #[payable]
    fn ft_transfer_call(
        &mut self,
        receiver_id: ValidAccountId,
        amount: U128,
        msg: String,
        memo: Option<String>,
    ) -> Promise {
        assert_one_yocto();
        let sender_id = env::predecessor_account_id();
        let amount = amount.into();
        self.internal_transfer(&sender_id, receiver_id.as_ref(), amount, memo);
        // Initiating receiver's call and the callback
        ext_fungible_token_receiver::ft_on_transfer(
            sender_id.clone(),
            amount.into(),
            msg,
            receiver_id.as_ref(),
            NO_DEPOSIT,
            env::prepaid_gas() - GAS_FOR_FT_TRANSFER_CALL,
        )
        .then(ext_self::ft_resolve_transfer(
            sender_id,
            receiver_id.into(),
            amount.into(),
            &env::current_account_id(),
            NO_DEPOSIT,
            GAS_FOR_RESOLVE_TRANSFER,
        ))
    }

    fn ft_total_supply(&self) -> U128 {
        self.total_supply.into()
    }

    fn ft_balance_of(&self, account_id: ValidAccountId) -> U128 {
        self.accounts.get(account_id.as_ref()).unwrap_or(0).into()
    }
}

#[near_bindgen]
impl FungibleTokenResolver for Contract {
    fn ft_resolve_transfer(
        &mut self,
        sender_id: AccountId,
        receiver_id: AccountId,
        amount: U128,
    ) -> U128 {
        assert_self();
        let amount: Balance = amount.into();

        // Get the unused amount from the `ft_on_transfer` call result.
        let unused_amount = match env::promise_result(0) {
            PromiseResult::NotReady => unreachable!(),
            PromiseResult::Successful(value) => {
                if let Ok(unused_amount) = near_sdk::serde_json::from_slice::<U128>(&value) {
                    std::cmp::min(amount, unused_amount.0)
                } else {
                    amount
                }
            }
            PromiseResult::Failed => amount,
        };

        if unused_amount > 0 {
            let receiver_balance = self.accounts.get(&receiver_id).unwrap_or(0);
            if receiver_balance > 0 {
                let refund_amount = std::cmp::min(receiver_balance, unused_amount);
                self.accounts
                    .insert(&receiver_id, &(receiver_balance - refund_amount));

                if let Some(sender_balance) = self.accounts.get(&sender_id) {
                    self.accounts
                        .insert(&sender_id, &(sender_balance + refund_amount));
                    env::log(
                        format!(
                            "Refund {} from {} to {}",
                            refund_amount, receiver_id, sender_id
                        )
                        .as_bytes(),
                    );
                    return (amount - refund_amount).into();
                } else {
                    // Sender's account was deleted, so we need to burn tokens.
                    self.total_supply -= refund_amount;
                    env::log(b"The account of the sender was deleted");
                    env::log(format!("Burn {}", refund_amount).as_bytes());
                }
            }
        }
        amount.into()
    }
}
