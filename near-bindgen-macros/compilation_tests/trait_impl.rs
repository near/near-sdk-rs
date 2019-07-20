//! Smart contract that implements trait.

#![feature(const_vec_new)]
use near_bindgen::near_bindgen;
use serde::{Deserialize, Serialize};

#[near_bindgen]
#[derive(Default, Serialize, Deserialize)]
struct Incrementer {
    value: u32,
}

pub trait Zeroable {
    fn set_to_zero(&mut self);
}

#[near_bindgen]
impl Incrementer {
    pub fn inc(&mut self, by: u32) {
        self.value += by;
    }
}

#[near_bindgen]
impl Zeroable for Incrementer {
    fn set_to_zero(&mut self) {
        self.value = 0;
    }
}

fn main() {}
