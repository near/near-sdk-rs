//! This module represents a Fungible Token standard.
//!
//! # Examples
//! See [`FungibleTokenCore`] and [`FungibleTokenResolver`] for example usage and [`FungibleToken`]
//! for core standard implementation.

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
