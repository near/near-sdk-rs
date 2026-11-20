//! `schemars` 1.x [`JsonSchema`] impls, behind the `schemars-v1` feature.
//!
//! The public types can't derive these next to their `schemars-v0_8` derives: both derives read
//! the same `#[schemars(crate = ...)]` attribute and fail with a duplicate when both features
//! are on, which `abi` plus `schemars-v1` does. So each type forwards to a private mirror that
//! derives the 1.x schema from the same serde attributes, with the field types (or the
//! `serde_with` adapters standing in for them) and doc comments of the original.
//!
//! The mirrors must change whenever the serde form or the docs of the originals do.
//! `tests/schemars_v1.rs` compares both versions' output when both features are on and fails
//! if they drift apart.

use std::borrow::Cow;

use schemars_v1::{JsonSchema, Schema, SchemaGenerator};

/// Forwards to the mirror of the same name, keeping the public type's name and id so that
/// `$defs` entries and `$ref`s name the public types.
macro_rules! forward_to_mirror {
    ($($ty:ident),* $(,)?) => {$(
        impl JsonSchema for crate::$ty {
            fn schema_name() -> Cow<'static, str> {
                Cow::Borrowed(stringify!($ty))
            }

            fn schema_id() -> Cow<'static, str> {
                Cow::Borrowed(concat!("near_global_contracts::", stringify!($ty)))
            }

            fn json_schema(generator: &mut SchemaGenerator) -> Schema {
                mirror::$ty::json_schema(generator)
            }
        }
    )*};
}

forward_to_mirror!(
    GlobalContractId,
    StateInit,
    StateInitV1,
    RawStateInit,
    UniversalStateInit,
    UniversalStateInitV1,
);

// Never constructed: only their derived schemas are used.
#[allow(dead_code)]
mod mirror {
    use std::collections::{BTreeMap, BTreeSet};

    use near_account_id::AccountId;
    use near_crypto_hash::CryptoHash;
    use near_sdk_core::types::PublicKeyHandle;
    use schemars_v1::JsonSchema;

    /// Identifies which [global contract] an account should run.
    ///
    /// Global contracts let contract code be deployed once and shared across the whole
    /// network, so many accounts can run the same code without each storing its own copy.
    /// A global contract is referenced in one of two ways: by hash of its code or by `AccountId` that deployed it.
    ///
    /// [global contract]: https://github.com/near/NEPs/pull/591
    #[derive(JsonSchema)]
    #[schemars(crate = "::schemars_v1")]
    pub enum GlobalContractId {
        /// Reference the contract by the hash of its code.
        ///
        /// This pins that exact bytecode: the reference never changes, even if the account
        /// that originally deployed the code later deploys something else. Use it when you
        /// want an immutable dependency.
        #[serde(rename = "hash")]
        CodeHash(
            #[schemars(with = "::serde_with::Schema<CryptoHash, ::serde_with::base58::Base58>")]
            CryptoHash,
        ),
        /// Reference the contract by the account that deployed it globally.
        ///
        /// This follows whatever code that account currently has deployed as a global
        /// contract, so the referenced code changes if the deployer upgrades it. Use it when
        /// you want to track the deployer's latest version.
        #[serde(rename = "account_id")]
        AccountId(AccountId),
    }

    /// Initial on-chain state used to derive a [deterministic account] ([NEP-616]).
    ///
    /// A deterministic account lives at an address computed from its own initial state, so
    /// anyone who knows the state can work out the address ahead of time. Enable the `borsh`
    /// feature and call `StateInit::derive_account_id` to get the resulting [`AccountId`].
    ///
    /// Version so the layout can evolve; [`V1`](StateInit::V1) is the only version today.
    ///
    /// [deterministic account]: https://github.com/near/NEPs/pull/616
    /// [NEP-616]: https://github.com/near/NEPs/pull/616
    /// [`AccountId`]: near_account_id::AccountId
    #[derive(JsonSchema)]
    #[schemars(crate = "::schemars_v1")]
    pub enum StateInit {
        /// Version 1: see [`StateInitV1`].
        V1(crate::StateInitV1),
    }

    /// Version 1 of the [`StateInit`] payload.
    ///
    /// Holds the global contract to run and the account's initial storage entries.
    #[derive(JsonSchema)]
    #[schemars(crate = "::schemars_v1")]
    pub struct StateInitV1 {
        /// The global contract whose code the derived account will run.
        code: crate::GlobalContractId,
        /// Key/value entries written to the account's trie storage at creation.
        #[schemars(with = "::serde_with::Schema<BTreeMap<Vec<u8>, Vec<u8>>, \
                           BTreeMap<::serde_with::base64::Base64, ::serde_with::base64::Base64>>")]
        data: BTreeMap<Vec<u8>, Vec<u8>>,
    }

    /// Borsh bytes of a [`UniversalStateInit`]: the form a `0u` universal account is created from and
    /// the only input its account id is derived from ([NEP-655]).
    ///
    /// The account id is SHA3-256 over exactly these bytes. Use
    /// `try_derive_account_id` (requires `borsh`) to check deserialization and canonical
    /// encoding before presenting an address for funding. Wrapping bytes in this type does not
    /// validate them. Non-canonical bytes must be rejected, not silently normalized to another id.
    /// Code forwarding a state init (a factory, relayer or wallet contract) must preserve the bytes.
    /// Opaque forwarding also works for versions this crate predates; local validation does not.
    ///
    /// Mirrors nearcore's `near_primitives_core::universal_state_init::RawStateInit`: borsh writes a
    /// `u32` length prefix and the bytes (how the action carries it as a field), and JSON is a single
    /// base64 string (how RPC shows it). Neither wrapper encoding is what the id hashes: that is
    /// `self.0` alone.
    ///
    /// Build one from a typed value with `UniversalStateInit::to_raw` or `From`, or wrap bytes you were
    /// handed with `RawStateInit::from(bytes)`. Decode one with `UniversalStateInit::try_from` (the
    /// typed conversions need the `borsh` feature).
    ///
    /// [NEP-655]: https://github.com/near/NEPs/pull/655
    #[derive(JsonSchema)]
    #[schemars(crate = "::schemars_v1")]
    pub struct RawStateInit(
        #[schemars(with = "::serde_with::Schema<Vec<u8>, ::serde_with::base64::Base64>")] Vec<u8>,
    );

    /// Versioned initial state of a `0u` universal account.
    ///
    /// A universal account ([NEP-655]) is a post-quantum-safe successor to implicit and deterministic
    /// accounts: its address is `0u` followed by the Crockford base32 encoding of the SHA3-256 hash of
    /// the borsh-serialized `UniversalStateInit` that creates it.
    ///
    /// This is a builder and a decoded view. What the chain carries, and what the id commits to, is
    /// the [`RawStateInit`] bytes. Encoding a typed value always gives the canonical bytes (sorted
    /// `BTree*` containers), which match what nearcore emits for the same logical value.
    ///
    /// The discriminant is the only version marker; new fields or semantics arrive as a new variant.
    ///
    /// With `serde`, the typed JSON form supports builders and inspection: snake_case variant tags,
    /// base64 storage keys and values, and string access-key handles. This does not establish that
    /// incoming raw bytes were canonical. Decoding and re-encoding can change their account id.
    /// Contracts forwarding state init should accept [`RawStateInit`] (base64 in JSON) and preserve
    /// its bytes, including versions this crate cannot decode.
    ///
    /// Contract authors reach this through `near-sdk`, which re-exports it under
    /// `near_sdk::universal_state_init` and adds `Promise::universal_state_init`. Off-chain code can
    /// derive the same id here without the SDK.
    ///
    /// [NEP-655]: https://github.com/near/NEPs/pull/655
    ///
    /// # Requirements
    ///
    /// On-chain use requires a host that supports universal accounts (nearcore protocol version 88+,
    /// shipped in nearcore 2.14).
    #[derive(JsonSchema)]
    #[schemars(crate = "::schemars_v1")]
    #[serde(rename_all = "snake_case")]
    pub enum UniversalStateInit {
        /// Version 1: see [`UniversalStateInitV1`].
        V1(crate::UniversalStateInitV1),
    }

    /// Version 1 of the [`UniversalStateInit`] payload.
    ///
    /// Which fields are populated decides the kind of account: a key-only account has no `code`, a
    /// contract account has one. A payload with neither code nor keys is valid too, but the resulting
    /// account can never be controlled.
    #[derive(JsonSchema)]
    #[schemars(crate = "::schemars_v1")]
    pub struct UniversalStateInitV1 {
        /// Contract code, or `None` for a key-only account.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        code: Option<crate::GlobalContractId>,
        /// Initial storage; empty unless seeded. Sorted keys give a canonical encoding.
        #[schemars(with = "::serde_with::Schema<BTreeMap<Vec<u8>, Vec<u8>>, \
                           BTreeMap<::serde_with::base64::Base64, ::serde_with::base64::Base64>>")]
        #[serde(default, skip_serializing_if = "::std::collections::BTreeMap::is_empty")]
        data: BTreeMap<Vec<u8>, Vec<u8>>,
        /// Full-access keys as compact on-trie handles. Sorted for a canonical encoding.
        #[serde(default, skip_serializing_if = "::std::collections::BTreeSet::is_empty")]
        access_keys: BTreeSet<PublicKeyHandle>,
    }
}
