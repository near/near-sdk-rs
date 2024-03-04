//! Impl block has type parameters.

use borsh::{BorshDeserialize, BorshSerialize};
use near_sdk::near_bindgen;
#[allow(unused_imports)]
use std::marker::PhantomData;

#[near(contract_state)]
#[derive(Default, BorshDeserialize, BorshSerialize)]
struct Incrementer<T> {
    value: u32,
    data: PhantomData<T>,
}

#[near]
impl<'a, T: 'a + std::fmt::Display> Incrementer<T> {
    pub fn inc(&mut self, by: u32) {
        self.value += by;
    }
}

fn main() {}
