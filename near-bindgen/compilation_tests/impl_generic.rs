//! Impl block has type parameters.

#![feature(const_vec_new)]
use near_bindgen::near_bindgen;
use serde::{Deserialize, Serialize};
use std::marker::PhantomData;

#[derive(Default, Serialize, Deserialize)]
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
