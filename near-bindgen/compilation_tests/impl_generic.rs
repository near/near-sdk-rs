//! Impl block has type parameters.

#![feature(const_vec_new)]
use near_bindgen::near_bindgen;
use borsh::{BorshDeserialize, BorshSerialize};
use std::marker::PhantomData;

#[near_bindgen]
#[derive(Default, BorshDeserialize, BorshSerialize)]
struct Incrementer<T> {
    value: u32,
    data: PhantomData<T>,
}

#[near_bindgen]
impl<'a, T: 'a + std::fmt::Display> Incrementer<T> {
    pub fn inc(&mut self, by: u32) {
        self.value += by;
    }
}

fn main() {}
