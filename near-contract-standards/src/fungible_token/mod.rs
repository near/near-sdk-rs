//! Fungible tokens as described in [by the spec](https://nomicon.io/Standards/Tokens/FungibleToken).
//!
//! This module represents a Fungible Token standard.
//!
//! # Examples
//! See [`FungibleTokenCore`] and [`FungibleTokenResolver`] for example usage and [`FungibleToken`]
//! for core standard implementation.
//!
//! # Cross-contract calls
//! The traits in this module are annotated with `#[ext_contract]`, which means they can also be
//! used for type-safe cross-contract calls to external contracts that implement these interfaces.
//! See the documentation on [`FungibleTokenCore`] for examples.

pub mod core;
pub mod core_impl;
pub mod events;
pub mod macros;
pub mod metadata;
pub mod receiver;
pub mod resolver;
pub mod storage_impl;

pub use crate::fungible_token::core::FungibleTokenCore;
pub use core_impl::{Balance, FungibleToken};
pub use resolver::FungibleTokenResolver;
