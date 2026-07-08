//! Global contract identifiers and deterministic account derivation as defined by
//! [NEP-616](https://github.com/near/NEPs/pull/616).
//!
//! ## When to use this crate directly
//!
//! - **Off-chain code** (indexers, CLIs, services) that needs to compute the deterministic
//!   account ID for a given [`StateInit`] without pulling in all of `near-sdk`.
//! - **Non-NEAR wasm runtimes** (e.g. TEE-hosted code) that want NEP-616 types and derivation
//!   without any contract-runtime dependencies.
//!
//! Contract authors using `near-sdk` get the same types re-exported under
//! [`near_sdk::state_init`](https://docs.rs/near-sdk/) and do not need to depend on this crate
//! directly.
//!
//! ## Feature flags
//!
//! By default this crate exposes only the type definitions. To use
//! [`StateInit::derive_account_id`] you must enable the `borsh` feature. The hashing
//! backend is then selected automatically by the
//! [`near-digest`](https://docs.rs/near-digest) crate:
//!
//! - On-chain contract builds: set `--cfg near` (the `cargo-near` toolchain does this
//!   automatically). Hashing routes through the `keccak256` host function.
//! - Off-chain / non-NEAR wasm builds: hashing is performed in pure Rust.
//!
//! Both paths produce identical output, so you can verify on-chain derivations off-chain.
//!
//! Other features:
//! - `serde`, `borsh` — derive the matching (de)serialization traits.
//! - `abi` — schema generation for ABI tooling.
//! - `arbitrary` — `arbitrary::Arbitrary` impls for fuzzing.
//! - `near-primitives-interop` — `From`/`Into` between this crate's types and the equivalents
//!   in `near-primitives-core`, for code that bridges to nearcore.
//!
//! ## Example: off-chain account ID derivation
//!
//! ```ignore
//! # // Requires the `borsh` feature.
//! use near_global_contracts::{StateInit, StateInitV1, GlobalContractId};
//!
//! let state_init = StateInit::from(StateInitV1::code(
//!     GlobalContractId::AccountId("example.near".parse().unwrap()),
//! ));
//! let account_id = state_init.derive_account_id();
//! println!("{account_id}"); // 0s<40 hex chars>
//! ```

mod global_contract_identifier;
pub use global_contract_identifier::*;

mod state_init;
pub use state_init::*;

// Re-export the underlying AccountId so consumers don't have to add `near-account-id` to their
// Cargo.toml just to spell out the return type of `derive_account_id`.
pub use near_account_id::AccountId;
