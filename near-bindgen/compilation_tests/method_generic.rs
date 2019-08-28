//! Method has type parameters.

#![feature(const_vec_new)]
use near_bindgen::near_bindgen;
use borsh::{BorshDeserialize, BorshSerialize};

#[near_bindgen]
#[derive(Default, BorshDeserialize, BorshSerialize)]
struct Incrementer {
    value: u32,
}

#[near_bindgen]
impl Incrementer {
    pub fn method<'a, T: 'a + std::fmt::Display>(&self) {
        unimplemented!()
    }
}

fn main() {}
