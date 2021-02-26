/*!
Some hypothetical DeFi contract that will do smart things with the transferred tokens
*/
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::json_types::{ValidAccountId, U128};
use near_sdk::{env, ext_contract, log, near_bindgen, setup_alloc, PromiseOrValue};

setup_alloc!();

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize)]
pub struct DeFi {}

impl Default for DeFi {
    fn default() -> Self {
        Self {}
    }
}

#[ext_contract(me)]
pub trait CallMyself {
    fn value_please(&self, amount_to_return: String) -> U128;
}

#[near_bindgen]
#[allow(unused_variables)]
impl DeFi {
    /// If given `msg: "take-my-money", immediately returns U128::From(0)
    /// Otherwise, makes a cross-contract call to own `value_please` function, passing `msg`
    /// value_please will attempt to parse `msg` as an integer and return a U128 version of it
    pub fn ft_on_transfer(
        &self,
        sender_id: ValidAccountId,
        amount: U128,
        msg: String,
    ) -> PromiseOrValue<U128> {
        log!("in ft_on_transfer, msg = {:?}", msg);
        match msg.as_str() {
            "take-my-money" => PromiseOrValue::Value(U128::from(0)),
            _ => {
                let prepaid_gas = env::prepaid_gas();
                let account_id = env::current_account_id();
                me::value_please(msg, &account_id, 0, prepaid_gas / 4).into()
            }
        }
    }

    pub fn value_please(&self, amount_to_return: String) -> U128 {
        log!("in value_please, amount_to_return = {:?}", amount_to_return);
        let num: u128 = amount_to_return.parse().unwrap();
        U128::from(num)
    }
}
