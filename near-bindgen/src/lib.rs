#![feature(const_vec_new)]
#[macro_use]
extern crate near_bindgen_macros;
pub use near_bindgen_macros::near_bindgen;

pub mod collections;
pub mod context;
pub mod environment;
pub use environment::ENV;
pub use environment::mocked_environment::MockedEnvironment;
