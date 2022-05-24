//! Testing that state with enum compiles correctly

use borsh::{BorshDeserialize, BorshSerialize};
use near_sdk::near_bindgen;

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize)]
enum StateMachine {
    StateA,
    StateB,
}

impl Default for StateMachine {
    fn default() -> Self {
        Self::StateA
    }
}

#[near_bindgen]
impl StateMachine {
    pub fn swap_state(&mut self) {
        *self = match self {
            Self::StateA => Self::StateB,
            Self::StateB => Self::StateA,
        };
    }
}

fn main() {}