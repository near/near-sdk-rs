// We want to enable all clippy lints, but some of them generate false positives.
#![allow(clippy::missing_const_for_fn, clippy::redundant_pub_crate)]

/// Non-fungible tokens as described in [by the spec](https://nomicon.io/Standards/NonFungibleToken/README.html).
pub mod non_fungible_token;

/// This upgrade standard is a use case where a staging area exists for a WASM
/// blob, allowing it to be stored for a period of time before deployed.
#[deprecated(
    since = "4.1.0",
    note = "This was removed because there is no standard (NEP) for upgradable contracts."
)]
pub mod upgrade;

pub(crate) mod event;

pub mod contract_metadata;
