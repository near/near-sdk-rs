//! Universal account (`0u`) state initialization types.
//!
//! A universal account is a post-quantum-safe successor to implicit and deterministic accounts:
//! its address is `0u` followed by the Crockford base32 encoding of the SHA3-256 hash of the
//! borsh-serialized [`UniversalStateInit`] that creates it.
//!
//! The types live in the [`near_global_contracts`](https://docs.rs/near-global-contracts) crate
//! ([`UniversalStateInit`], [`UniversalStateInitV1`]) and in
//! [`near_sdk_core`](https://docs.rs/near-sdk-core) ([`PublicKeyHandle`], next to
//! [`PublicKey`](crate::PublicKey)); they are re-exported here for convenience.
//!
//! Universal accounts are created with [`Promise::universal_state_init`](crate::Promise::universal_state_init)
//! (or the lower-level [`env::promise_batch_action_universal_state_init`](crate::env::promise_batch_action_universal_state_init)),
//! and their id can be computed ahead of time with [`UniversalStateInit::derive_account_id`] or
//! the [`env::universal_state_init_to_account_id`](crate::env::universal_state_init_to_account_id)
//! host function.
//!
//! # Requirements
//!
//! Requires the host to support universal accounts (nearcore protocol version 87+, shipped in
//! nearcore 2.14).

pub use near_global_contracts::universal_state_init::{UniversalStateInit, UniversalStateInitV1};
pub use near_sdk_core::types::{ParsePublicKeyError, PublicKeyHandle};
