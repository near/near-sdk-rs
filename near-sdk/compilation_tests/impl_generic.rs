//! Impl block has type parameters.

use near_sdk::near;
#[allow(unused_imports)]
use std::marker::PhantomData;

#[near(contract_state)]
#[derive(Default)]
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
