#![feature(const_vec_new)]
#[macro_use]
extern crate near_bindgen_macros;
pub use near_bindgen_macros::near_bindgen;

mod binding;
pub use crate::binding::*;

mod header;
pub use crate::header::*;
