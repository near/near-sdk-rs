//! `examples/fungible-token` upgraded in place to storage v2 (storage v2 prototype,
//! PROTOTYPE_NOTES.md "Migration round").
//!
//! - [`v2`]: "move the root". The deployed v1 `FungibleToken` and `LazyOption` stay as root
//!   fields with their old prefixes; `migrate` reads the old Borsh `STATE` blob once. One new
//!   v2 field.
//! - [`v3`]: "rekey when worth it". `FungibleToken` is flattened so its accounts map can be
//!   wrapped in `Migrating<collections::LookupMap, state::LookupMap>`; `migrate` reads the v2
//!   root's raw tagged fields.
//!
//! A wasm build exports `v2` by default and `v3` with `--features v3`. Native builds compile
//! both (no exports), so tests can run them over the same storage.
//!
//! v1 types live only at the root, as `#[key(n, legacy)]` fields (API round 3, T7). A
//! `FungibleToken` nested in a `#[near(storage)]` record used to compile, and `remove` on the
//! record orphaned every balance (the round-2 `tests/nested_trap.rs`). It is now a compile
//! error: `FungibleToken` holds a `collections::LookupMap` and is not a storage v2 value.
//!
//! ```compile_fail,E0277
//! use near_contract_standards::fungible_token::FungibleToken;
//! use near_sdk::near;
//! use near_sdk::state::Map;
//!
//! #[near(storage)]
//! pub struct Pool {
//!     // error[E0277]: `FungibleToken` is not a storage v2 value
//!     #[key(1, required)]
//!     pub token: FungibleToken,
//! }
//!
//! #[near(contract_state(layout = v2))]
//! pub struct Pools {
//!     #[key(1)]
//!     pub pools: Map<u32, Pool>,
//! }
//! ```
//!
//! At the root, the same type is accepted only when marked `legacy`:
//!
//! ```
//! use near_contract_standards::fungible_token::FungibleToken;
//! use near_sdk::near;
//!
//! #[near(contract_state(layout = v2))]
//! pub struct Single {
//!     #[key(1, legacy)]
//!     pub token: FungibleToken,
//! }
//! ```

#[cfg(any(not(target_arch = "wasm32"), not(feature = "v3")))]
pub mod v2;
#[cfg(any(not(target_arch = "wasm32"), feature = "v3"))]
pub mod v3;
