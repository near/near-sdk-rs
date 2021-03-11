//! # near_sdk_sim
//!
//! This crate provides an interface for simulating transactions on NEAR's Blockchain.
//! The simulator uses a standalone runtime that can handle any of the [actions](https://nomicon.io/RuntimeSpec/Actions.html) provided by the
//! real runtime, including: creating accounts, deploying contracts, making contract calls and
//! calling view methods.

pub mod outcome;
#[doc(inline)]
pub use outcome::*;
mod cache;
pub mod runtime;
pub mod units;
pub mod user;
pub use near_crypto;
#[doc(hidden)]
pub use near_primitives::*;
#[doc(inline)]
pub use units::*;
#[doc(inline)]
pub use user::*;

#[doc(hidden)]
pub use lazy_static_include;
