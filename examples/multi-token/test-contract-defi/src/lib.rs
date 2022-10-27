/*!
Some hypothetical DeFi contract that will do smart things with the transferred tokens
*/
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::json_types::U128;
use near_sdk::{
    env, log, near_bindgen, require, AccountId, Balance, Gas, PanicOnDefault,
    PromiseOrValue,
};
use near_contract_standards::multi_token::core::MultiTokenReceiver;
use near_contract_standards::multi_token::token::TokenId;

const BASE_GAS: u64 = 5_000_000_000_000;
const PROMISE_CALL: u64 = 5_000_000_000_000;
const GAS_FOR_MT_ON_TRANSFER: Gas = Gas(BASE_GAS + PROMISE_CALL);

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct DeFi {
    multi_token_account_id: AccountId,
}

// Have to repeat the same trait for our own implementation.
trait ValueReturnTrait {
    fn value_please(
        &self,
        num_tokens: usize,
        amount_to_return: String,
    ) -> PromiseOrValue<Vec<U128>>;
}

#[near_bindgen]
impl DeFi {
    #[init]
    pub fn new(multi_token_account_id: AccountId) -> Self {
        require!(!env::state_exists(), "Already initialized");
        Self { multi_token_account_id }
    }
}

#[near_bindgen]
impl MultiTokenReceiver for DeFi {
    /// If given `msg: "take-my-money", immediately returns U128::From(0)
    /// Otherwise, makes a cross-contract call to own `value_please` function, passing `msg`
    /// value_please will attempt to parse `msg` as an integer and return a vec of
    /// token_ids.len() many copies of the U128 version of it.
    fn mt_on_transfer(
        &mut self,
        sender_id: AccountId,
        previous_owner_ids: Vec<AccountId>,
        token_ids: Vec<TokenId>,
        amounts: Vec<U128>,
        msg: String,
    ) -> PromiseOrValue<Vec<U128>> {
        // Verifying that we were called by multi-token contract that we expect.
        require!(
            env::predecessor_account_id() == self.multi_token_account_id,
            "Only supports the one multi-token contract"
        );

        log!(
            "received {} types of tokens from @{} mt_on_transfer, msg = {}, previous_owner_ids = {:?}",
            token_ids.len(),
            sender_id.as_ref(),
            msg,
            previous_owner_ids
        );

        for i in 0..token_ids.len() {
            log!("-> {} of token {}", token_ids[i], amounts[i].0)
        }

        match msg.as_str() {
            "take-my-money" => PromiseOrValue::Value(vec![U128::from(0); token_ids.len()]),
            _ => {
                let prepaid_gas = env::prepaid_gas();
                let account_id = env::current_account_id();
                Self::ext(account_id)
                    .with_static_gas(prepaid_gas - GAS_FOR_MT_ON_TRANSFER)
                    .value_please(token_ids.len(), msg)
                    .into()
            }
        }
    }
}

#[near_bindgen]
impl ValueReturnTrait for DeFi {
    fn value_please(
        &self,
        num_tokens: usize,
        amount_to_return: String,
    ) -> PromiseOrValue<Vec<U128>> {
        log!("in value_please, amount_to_return = {}", amount_to_return);
        let amount: Balance = amount_to_return.parse().expect("Not an integer");
        PromiseOrValue::Value(vec![amount.into(); num_tokens])
    }
}
