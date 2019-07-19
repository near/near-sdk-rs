#![feature(const_vec_new)]
#[macro_use]
extern crate near_bindgen_macros;
pub use near_bindgen_macros::near_bindgen;

pub mod collections;
pub mod context;
pub use context::CONTEXT;
