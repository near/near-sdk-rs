//! Global contract identifiers, plus the state inits and account id derivation of deterministic
//! `0s` accounts ([NEP-616]) and universal `0u` accounts ([NEP-655]).
//!
//! - [`GlobalContractId`] names a global contract by code hash or by deployer account.
//! - [`StateInit`] creates a deterministic `0s` account: Keccak-256 of its borsh, hex encoded.
//! - [`RawStateInit`] creates a universal `0u` account: SHA3-256 of exactly these bytes, Crockford
//!   base32 encoded. [`UniversalStateInit`] builds and decodes the bytes, but the bytes are what
//!   the id commits to, so forward them unchanged instead of re-encoding a decoded value.
//!
//! ## When to use this crate directly
//!
//! - **Off-chain code** (indexers, CLIs, services) that needs to compute the `0s` or `0u` account
//!   ID for a state init without pulling in all of `near-sdk`.
//! - **Non-NEAR wasm runtimes** (e.g. TEE-hosted code) that want these types and derivations
//!   without any contract-runtime dependencies.
//!
//! Contract authors using `near-sdk` get the same types re-exported under
//! [`near_sdk::state_init`](https://docs.rs/near-sdk/) and
//! [`near_sdk::universal_state_init`](https://docs.rs/near-sdk/) and do not need to depend on this
//! crate directly.
//!
//! ## Feature flags
//!
//! By default this crate exposes only the type definitions. To use
//! `StateInit::derive_account_id` you must enable the `borsh` feature. The hashing
//! backend is then selected automatically:
//!
//! - On-chain contract builds: set `--cfg near` (the `cargo-near` toolchain does this
//!   automatically). Hashing routes through the `keccak256` host function via `near-sdk-env`.
//! - Off-chain / non-NEAR wasm builds: hashing is performed in pure Rust via [`sha3`],
//!   pulled in unconditionally on the `cfg(not(near))` path.
//!
//! Both paths produce identical output, so you can verify on-chain derivations off-chain.
//!
//! The `0u` derivation ([`RawStateInit::derive_account_id`], [`derive_universal_account_id`])
//! needs no feature and works the same way with SHA3-256 and the `sha3_256` host function.
//! Encoding or decoding a [`UniversalStateInit`] needs `borsh`.
//!
//! Other features:
//! - `serde`, `borsh` — derive the matching (de)serialization traits. [`RawStateInit`] is a base64
//!   string in JSON, like nearcore's. The typed [`UniversalStateInit`] has a JSON form too, but it
//!   re-encodes to the canonical bytes, so forward a [`RawStateInit`] instead.
//! - `abi` — schema generation for ABI tooling. Like `schemars-v0_8`, it enables `serde`, since
//!   the schema describes the JSON form.
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
//!
//! ## Example: `0u` account ID of state-init bytes
//!
//! ```ignore
//! # // Encoding the typed value requires the `borsh` feature.
//! use near_global_contracts::{GlobalContractId, RawStateInit, UniversalStateInitV1};
//!
//! let raw = RawStateInit::from(
//!     UniversalStateInitV1::default()
//!         .with_code(GlobalContractId::AccountId("code.near".parse().unwrap())),
//! );
//! println!("{}", raw.derive_account_id()); // 0u<52 base32 chars>
//! ```
//!
//! [NEP-616]: https://github.com/near/NEPs/pull/616
//! [NEP-655]: https://github.com/near/NEPs/pull/655

#[cfg(all(test, feature = "__near-sdk-unit-testing"))]
// XXX: `near-sdk` was added in order to enable doctests and tests that require mockchain
use near_sdk as _;

mod global_contract_identifier;
pub use global_contract_identifier::*;

mod state_init;
pub use state_init::*;

mod universal_state_init;
pub use universal_state_init::*;

// Re-export the underlying AccountId so consumers don't have to add `near-account-id` to their
// Cargo.toml just to spell out the return type of `derive_account_id`.
pub use near_account_id::AccountId;

// `UniversalStateInitV1::access_keys` holds these; re-exported so off-chain users can build a
// state init without a direct `near-sdk-core` dependency.
pub use near_sdk_core::types::PublicKeyHandle;
