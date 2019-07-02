//! Method has type parameters.

#![feature(const_vec_new)]
use near_bindgen::near_bindgen;
use serde::{Deserialize, Serialize};

#[derive(Default, Serialize, Deserialize)]
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
