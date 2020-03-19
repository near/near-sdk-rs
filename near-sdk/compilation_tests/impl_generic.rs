//! Impl block has type parameters.

use near_sdk::near_sdk;
use borsh::{BorshDeserialize, BorshSerialize};
use std::marker::PhantomData;

#[near_sdk]
#[derive(Default, BorshDeserialize, BorshSerialize)]
struct Incrementer<T> {
    value: u32,
    data: PhantomData<T>,
}

#[near_sdk]
impl<'a, T: 'a + std::fmt::Display> Incrementer<T> {
    pub fn inc(&mut self, by: u32) {
        self.value += by;
    }
}

fn main() {}
