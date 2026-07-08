//! Deterministic account state initialization types ([NEP-616](https://github.com/near/NEPs/pull/616)).
//!
//! This module is available when the `deterministic-account-ids` feature is enabled on
//! `near-sdk`. The underlying types live in the
//! [`near_global_contracts`](https://docs.rs/near-global-contracts) crate and are re-exported
//! here for convenience.
//!
//! When invoked from inside an on-chain contract (i.e. a build that sets `--cfg near` via
//! `cargo-near`), `StateInit::derive_account_id` routes through the `keccak256` host
//! function; off-chain it falls back to a pure-Rust implementation. Off-chain consumers can
//! instead depend on `near-global-contracts` directly with the `borsh` feature.

pub use near_global_contracts::{StateInit, StateInitV1};
