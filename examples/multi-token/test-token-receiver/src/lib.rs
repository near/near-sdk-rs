/*!
A stub contract that implements mt_on_transfer for simulation testing mt_transfer_call.
*/
use near_contract_standards::multi_token::core::MultiTokenReceiver;
use near_contract_standards::multi_token::TokenId;
use near_sdk::json_types::U128;
use near_sdk::{env, log, near, require, AccountId, Gas, PanicOnDefault, PromiseOrValue};

/// It is estimated that we need to attach 5 TGas for the code execution and 5 TGas for cross-contract call
const GAS_FOR_MT_ON_TRANSFER: Gas = Gas::from_tgas(10);

#[near(contract_state)]
#[derive(PanicOnDefault)]
pub struct TokenReceiver {
    multi_token_account_id: AccountId,
}

// Have to repeat the same trait for our own implementation.
pub trait ValueReturnTrait {
    fn ok_go(&self, amounts: Vec<U128>) -> PromiseOrValue<Vec<U128>>;
}

#[near]
impl TokenReceiver {
    #[init]
    pub fn new(multi_token_account_id: AccountId) -> Self {
        Self { multi_token_account_id }
    }
}

#[near]
impl MultiTokenReceiver for TokenReceiver {
    /// Returns the amounts that should be returned to the previous owners.
    /// Several supported `msg` values:
    /// * "return-all-now" - immediately return all tokens
    /// * "keep-all-now" - immediately keep all tokens (return empty amounts)
    /// * "return-half-now" - immediately return half of each token amount
    /// * "return-all-later" - make cross-contract call which resolves with full refund
    /// * "keep-all-later" - make cross-contract call which resolves with zero refund
    /// Otherwise panics, which should also return tokens to the previous owners
    fn mt_on_transfer(
        &mut self,
        sender_id: AccountId,
        previous_owner_ids: Vec<AccountId>,
        token_ids: Vec<TokenId>,
        amounts: Vec<U128>,
        msg: String,
    ) -> PromiseOrValue<Vec<U128>> {
        // Verifying that we were called by the multi token contract that we expect.
        require!(
            env::predecessor_account_id() == self.multi_token_account_id,
            "Only supports the one multi token contract"
        );

        log!(
            "in mt_on_transfer; sender_id={}, num_tokens={}, msg={}",
            &sender_id,
            token_ids.len(),
            &msg
        );

        for (i, (token_id, amount)) in token_ids.iter().zip(amounts.iter()).enumerate() {
            log!(
                "  token[{}]: id={}, amount={}, previous_owner={}",
                i,
                token_id,
                amount.0,
                &previous_owner_ids[i]
            );
        }

        match msg.as_str() {
            "return-all-now" => {
                // Return all tokens to previous owners
                PromiseOrValue::Value(amounts)
            }
            "return-half-now" => {
                // Return half of each amount
                let half_amounts: Vec<U128> = amounts.iter().map(|a| U128(a.0 / 2)).collect();
                PromiseOrValue::Value(half_amounts)
            }
            "keep-all-now" => {
                // Keep all tokens (return zeros)
                let zeros: Vec<U128> = amounts.iter().map(|_| U128(0)).collect();
                PromiseOrValue::Value(zeros)
            }
            "return-all-later" => {
                let prepaid_gas = env::prepaid_gas();
                let account_id = env::current_account_id();
                Self::ext(account_id)
                    .with_static_gas(prepaid_gas.saturating_sub(GAS_FOR_MT_ON_TRANSFER))
                    .ok_go(amounts)
                    .into()
            }
            "keep-all-later" => {
                let zeros: Vec<U128> = amounts.iter().map(|_| U128(0)).collect();
                let prepaid_gas = env::prepaid_gas();
                let account_id = env::current_account_id();
                Self::ext(account_id)
                    .with_static_gas(prepaid_gas.saturating_sub(GAS_FOR_MT_ON_TRANSFER))
                    .ok_go(zeros)
                    .into()
            }
            _ => env::panic_str("unsupported msg"),
        }
    }
}

#[near]
impl ValueReturnTrait for TokenReceiver {
    fn ok_go(&self, amounts: Vec<U128>) -> PromiseOrValue<Vec<U128>> {
        log!("in ok_go, returning {} amounts", amounts.len());
        PromiseOrValue::Value(amounts)
    }
}
