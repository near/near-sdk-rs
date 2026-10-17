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

#[cfg(any(not(target_arch = "wasm32"), not(feature = "v3")))]
pub mod v2;
#[cfg(any(not(target_arch = "wasm32"), feature = "v3"))]
pub mod v3;
