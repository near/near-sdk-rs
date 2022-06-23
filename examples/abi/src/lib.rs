use near_sdk::__private::schemars::JsonSchema;
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::near_bindgen;
use near_sdk::serde::{Deserialize, Serialize};

#[derive(JsonSchema, Serialize, Deserialize)]
pub struct Pair(u32, u32);

#[derive(JsonSchema, Serialize, Deserialize)]
pub struct DoublePair {
    first: Pair,
    second: Pair,
}

#[near_bindgen]
#[derive(Default, BorshDeserialize, BorshSerialize)]
pub struct Adder {}

#[near_bindgen]
impl Adder {
    pub fn add(&self, a: Pair, b: Pair) -> Pair {
        sum_pair(&a, &b)
    }

    pub fn add_callback(
        &self,
        #[callback_unwrap] a: DoublePair,
        #[callback_unwrap] b: DoublePair,
        #[callback_vec] others: Vec<DoublePair>,
    ) -> DoublePair {
        Some(b).iter().chain(others.iter()).fold(a, |acc, el| DoublePair {
            first: sum_pair(&acc.first, &el.first),
            second: sum_pair(&acc.second, &el.second),
        })
    }
}

fn sum_pair(a: &Pair, b: &Pair) -> Pair {
    Pair(a.0 + b.0, a.1 + b.1)
}
