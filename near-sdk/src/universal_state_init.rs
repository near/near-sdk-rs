//! Universal account (`0u`) state initialization types ([NEP-655]).
//!
//! A universal account is a post-quantum-safe successor to implicit and deterministic accounts:
//! its address is `0u` followed by the Crockford base32 encoding of the SHA3-256 hash of the
//! borsh-serialized state init that creates it.
//!
//! [`RawStateInit`] holds those bytes. It is what the action carries, what the host functions
//! take and what the id commits to, so it is also the type to accept in contract arguments (JSON
//! as base64) and to forward unchanged. [`UniversalStateInit`] and [`UniversalStateInitV1`] build
//! and decode it. Their own JSON form re-encodes to the canonical bytes, and a non-canonical
//! encoding of the same logical state init is a different account, so never decode and re-encode
//! bytes you were handed.
//!
//! The types live in the [`near_global_contracts`](https://docs.rs/near-global-contracts) crate
//! ([`RawStateInit`], [`UniversalStateInit`], [`UniversalStateInitV1`]) and in
//! [`near_sdk_core`](https://docs.rs/near-sdk-core) ([`PublicKeyHandle`], next to
//! [`PublicKey`](crate::PublicKey)); they are re-exported here for convenience.
//!
//! Universal accounts are created with
//! [`Promise::universal_state_init`](crate::Promise::universal_state_init) (or the lower-level
//! [`env::promise_batch_action_universal_state_init`](crate::env::promise_batch_action_universal_state_init))
//! on a promise whose receiver is the id of the same bytes, computed with
//! [`RawStateInit::derive_account_id`] or the
//! [`env::universal_state_init_to_account_id`](crate::env::universal_state_init_to_account_id)
//! host function.
//!
//! # Requirements
//!
//! Requires the host to support universal accounts (nearcore protocol version 87+, shipped in
//! nearcore 2.14).
//!
//! [NEP-655]: https://github.com/near/NEPs/pull/655

pub use near_global_contracts::{RawStateInit, UniversalStateInit, UniversalStateInitV1};
pub use near_sdk_core::types::{ParsePublicKeyError, PublicKeyHandle};
