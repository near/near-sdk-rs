use std::collections::{BTreeMap, BTreeSet};

use near_account_id::UniversalAccountId;
use near_sdk_core::types::PublicKeyHandle;

use crate::GlobalContractId;

#[cfg(feature = "serde")]
use serde_with::base64::Base64;

/// Borsh bytes of a [`UniversalStateInit`]: the form a `0u` universal account is created from and
/// the only input its account id is derived from ([NEP-655]).
///
/// The account id is SHA3-256 over exactly these bytes ([`derive_account_id`](Self::derive_account_id)).
/// The chain accepts non-canonical encodings (unsorted or duplicate map keys, unsorted key sets) and
/// derives a *different* account for them than for the canonical encoding of the same logical
/// value. So code that forwards a state init it did not build itself (a factory, relayer or wallet
/// contract taking user input) must pass these bytes through as they are, never decode and
/// re-encode them. Passing bytes through also works for a state-init version this crate predates.
///
/// Mirrors nearcore's `near_primitives_core::universal_state_init::RawStateInit`: borsh writes a
/// `u32` length prefix and the bytes (how the action carries it as a field), and JSON is a single
/// base64 string (how RPC shows it). Neither wrapper encoding is what the id hashes: that is
/// `self.0` alone.
///
/// Build one from a typed value with [`UniversalStateInit::to_raw`] (or `From`), or wrap bytes you
/// were handed with `RawStateInit::from(bytes)`.
///
/// [NEP-655]: https://github.com/near/NEPs/pull/655
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(
    feature = "serde",
    cfg_eval::cfg_eval,
    serde_with::serde_as,
    derive(serde::Serialize, serde::Deserialize)
)]
#[cfg_attr(feature = "borsh", derive(borsh::BorshSerialize, borsh::BorshDeserialize))]
#[cfg_attr(
    feature = "schemars-v0_8",
    derive(::schemars_v0_8::JsonSchema),
    schemars(crate = "::schemars_v0_8")
)]
#[cfg_attr(feature = "abi", derive(borsh::BorshSchema))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
pub struct RawStateInit(#[cfg_attr(feature = "serde", serde_as(as = "Base64"))] pub Vec<u8>);

impl RawStateInit {
    /// The `0u` account id these bytes create: SHA3-256 of exactly `self.0`, Crockford-base32
    /// encoded. Same as [`derive_universal_account_id`].
    #[inline]
    pub fn derive_account_id(&self) -> near_account_id::AccountId {
        derive_universal_account_id(&self.0)
    }
}

impl AsRef<[u8]> for RawStateInit {
    #[inline]
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl From<Vec<u8>> for RawStateInit {
    #[inline]
    fn from(bytes: Vec<u8>) -> Self {
        Self(bytes)
    }
}

impl From<RawStateInit> for Vec<u8> {
    #[inline]
    fn from(raw: RawStateInit) -> Self {
        raw.0
    }
}

#[cfg(feature = "borsh")]
impl From<&UniversalStateInit> for RawStateInit {
    #[inline]
    fn from(state_init: &UniversalStateInit) -> Self {
        state_init.to_raw()
    }
}

#[cfg(feature = "borsh")]
impl From<UniversalStateInit> for RawStateInit {
    #[inline]
    fn from(state_init: UniversalStateInit) -> Self {
        state_init.to_raw()
    }
}

#[cfg(feature = "borsh")]
impl From<UniversalStateInitV1> for RawStateInit {
    #[inline]
    fn from(state_init: UniversalStateInitV1) -> Self {
        UniversalStateInit::V1(state_init).to_raw()
    }
}

#[cfg(feature = "near-primitives-interop")]
const _: () = {
    use near_primitives_core::universal_state_init::RawStateInit as NearcoreRawStateInit;

    impl From<NearcoreRawStateInit> for RawStateInit {
        #[inline]
        fn from(NearcoreRawStateInit(bytes): NearcoreRawStateInit) -> Self {
            Self(bytes)
        }
    }

    impl From<RawStateInit> for NearcoreRawStateInit {
        #[inline]
        fn from(RawStateInit(bytes): RawStateInit) -> Self {
            Self(bytes)
        }
    }
};

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
/// JSON deserialized into this type re-encodes to the canonical bytes, but the id commits to the
/// bytes, so anything forwarding a state init it did not build must carry the [`RawStateInit`],
/// not the typed value.
///
/// Contract authors reach this through `near-sdk`, which re-exports it under
/// `near_sdk::universal_state_init` and adds `Promise::universal_state_init`. Off-chain code can
/// derive the same id here without the SDK.
///
/// [NEP-655]: https://github.com/near/NEPs/pull/655
///
/// # Requirements
///
/// On-chain use requires a host that supports universal accounts (nearcore protocol version 87+,
/// shipped in nearcore 2.14).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "snake_case")
)]
#[cfg_attr(
    feature = "borsh",
    derive(borsh::BorshSerialize, borsh::BorshDeserialize),
    borsh(use_discriminant = true)
)]
#[cfg_attr(
    feature = "schemars-v0_8",
    derive(::schemars_v0_8::JsonSchema),
    schemars(crate = "::schemars_v0_8")
)]
#[cfg_attr(feature = "abi", derive(borsh::BorshSchema))]
#[repr(u8)]
pub enum UniversalStateInit {
    /// Version 1: see [`UniversalStateInitV1`].
    V1(UniversalStateInitV1) = 0,
}

/// Version 1 of the [`UniversalStateInit`] payload.
///
/// Which fields are populated decides the kind of account: a key-only account has no `code`, a
/// contract account has one. A payload with neither code nor keys is valid too, but the resulting
/// account can never be controlled.
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(
    feature = "serde",
    cfg_eval::cfg_eval,
    serde_with::serde_as,
    derive(serde::Serialize, serde::Deserialize)
)]
#[cfg_attr(feature = "borsh", derive(borsh::BorshSerialize, borsh::BorshDeserialize))]
#[cfg_attr(
    feature = "schemars-v0_8",
    derive(::schemars_v0_8::JsonSchema),
    schemars(crate = "::schemars_v0_8")
)]
#[cfg_attr(feature = "abi", derive(borsh::BorshSchema))]
pub struct UniversalStateInitV1 {
    /// Contract code, or `None` for a key-only account.
    #[cfg_attr(feature = "serde", serde(default, skip_serializing_if = "Option::is_none"))]
    pub code: Option<GlobalContractId>,
    /// Initial storage; empty unless seeded. Sorted keys give a canonical encoding.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "BTreeMap::is_empty"),
        serde_as(as = "BTreeMap<Base64, Base64>")
    )]
    pub data: BTreeMap<Vec<u8>, Vec<u8>>,
    /// Full-access keys as compact on-trie handles. Sorted for a canonical encoding.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "::std::collections::BTreeSet::is_empty")
    )]
    pub access_keys: BTreeSet<PublicKeyHandle>,
}

impl UniversalStateInitV1 {
    /// Sets the global contract the account will run.
    #[inline]
    pub fn with_code(mut self, code: impl Into<GlobalContractId>) -> Self {
        self.code = Some(code.into());
        self
    }

    /// Adds one key/value entry to the initial storage, returning `self` so calls can be
    /// chained.
    #[inline]
    pub fn with_data_entry(mut self, key: impl Into<Vec<u8>>, value: impl Into<Vec<u8>>) -> Self {
        self.data.insert(key.into(), value.into());
        self
    }

    /// Adds a full-access key. Accepts a `PublicKey` (ML-DSA-65 keys are hashed into their
    /// on-trie handle) or a ready-made [`PublicKeyHandle`].
    #[inline]
    pub fn with_access_key(mut self, key: impl Into<PublicKeyHandle>) -> Self {
        self.access_keys.insert(key.into());
        self
    }
}

impl From<UniversalStateInitV1> for UniversalStateInit {
    #[inline]
    fn from(state_init: UniversalStateInitV1) -> Self {
        Self::V1(state_init)
    }
}

impl UniversalStateInit {
    /// Contract code, or `None` for a key-only account.
    pub fn code(&self) -> Option<&GlobalContractId> {
        match self {
            Self::V1(inner) => inner.code.as_ref(),
        }
    }

    /// Initial storage entries.
    pub fn data(&self) -> &BTreeMap<Vec<u8>, Vec<u8>> {
        match self {
            Self::V1(inner) => &inner.data,
        }
    }

    /// Full-access keys.
    pub fn access_keys(&self) -> &BTreeSet<PublicKeyHandle> {
        match self {
            Self::V1(inner) => &inner.access_keys,
        }
    }

    /// Canonical borsh encoding of this state init, like nearcore's `UniversalStateInit::to_raw`.
    ///
    /// # Availability
    ///
    /// Requires the `borsh` feature.
    #[cfg(feature = "borsh")]
    pub fn to_raw(&self) -> RawStateInit {
        RawStateInit(borsh::to_vec(self).unwrap_or_else(|_| unreachable!()))
    }

    /// Decodes raw state-init bytes, like nearcore's `UniversalStateInit::from_raw`.
    ///
    /// Trailing bytes, malformed bytes and unknown versions are rejected. A non-canonical encoding
    /// decodes fine, but the account id commits to the original bytes, so re-encoding the result
    /// can give a different id. Keep the [`RawStateInit`] to forward it or to derive its id.
    ///
    /// Decoding can be stricter than the chain: borsh's `de_strict_order` cargo feature, if any
    /// crate in the dependency graph enables it, makes this reject unsorted or duplicate keys that
    /// nearcore accepts. Account ids are unaffected, since they never go through decoding.
    ///
    /// # Availability
    ///
    /// Requires the `borsh` feature.
    #[cfg(feature = "borsh")]
    pub fn from_raw(raw: &RawStateInit) -> Result<Self, std::io::Error> {
        borsh::from_slice(&raw.0)
    }

    /// The `0u` account id of this state init's canonical encoding. Shorthand for
    /// `self.to_raw().derive_account_id()`.
    ///
    /// The id commits to the bytes, not the logical value. A contract that forwards a state init
    /// it was handed must derive the id from those bytes ([`RawStateInit::derive_account_id`]) and
    /// forward them unchanged, never decode and re-encode.
    ///
    /// # Availability
    ///
    /// Requires the `borsh` feature. See [`derive_universal_account_id`] for the SHA3-256
    /// backend.
    #[cfg(feature = "borsh")]
    pub fn derive_account_id(&self) -> near_account_id::AccountId {
        self.to_raw().derive_account_id()
    }
}

/// Derives the `0u` account id from raw state-init bytes, like nearcore's
/// `derive_universal_account_id`: SHA3-256 over exactly those bytes, Crockford-base32 encoded.
///
/// Takes the bytes rather than a typed value, so a caller can pass through a state-init version
/// this crate predates.
///
/// SHA3-256 comes from [`near_digest::sha3::Sha3_256`]: the `sha3_256` host function in contract
/// builds (`--cfg near`, set by `cargo-near`) and pure Rust elsewhere, with identical output.
pub fn derive_universal_account_id(state_init: impl AsRef<[u8]>) -> near_account_id::AccountId {
    // Non-generic body, so each caller's argument type does not get its own copy.
    fn derive(state_init: &[u8]) -> near_account_id::AccountId {
        use near_digest::Digest;

        let hash: [u8; 32] = near_digest::sha3::Sha3_256::digest(state_init).into();
        UniversalAccountId::from_hash(hash).into_account_id()
    }
    derive(state_init.as_ref())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Known-answer vectors from nearcore's `universal_account_id.rs`.
    const UAID_KATS: &[([u8; 32], &str)] = &[
        ([0x00; 32], "0u0000000000000000000000000000000000000000000000000000"),
        ([0xff; 32], "0uzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzg"),
        (
            [
                0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d,
                0x0e, 0x0f, 0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1a, 0x1b,
                0x1c, 0x1d, 0x1e, 0x1f,
            ],
            "0u000g40r40m30e209185gr38e1w8124gk2gahc5rr34d1p70x3rfg",
        ),
        (
            [
                0x00, 0x07, 0x0e, 0x15, 0x1c, 0x23, 0x2a, 0x31, 0x38, 0x3f, 0x46, 0x4d, 0x54, 0x5b,
                0x62, 0x69, 0x70, 0x77, 0x7e, 0x85, 0x8c, 0x93, 0x9a, 0xa1, 0xa8, 0xaf, 0xb6, 0xbd,
                0xc4, 0xcb, 0xd2, 0xd9,
            ],
            "0u003gw58w4cn32e1z8s6n8pv2d5r7ezm5hj9sn8d8nyvbvh6btbcg",
        ),
    ];

    /// The id is consensus-critical, so the vectors stay here even though `near-account-id`
    /// owns the encoder: a change on either side has to show up in both places.
    #[test]
    fn encoder_matches_nearcore_known_answers() {
        for (hash, expected) in UAID_KATS {
            let account_id = UniversalAccountId::from_hash(*hash);
            assert_eq!(account_id.as_str(), *expected);
            assert_eq!(account_id.to_string(), *expected);
            assert_eq!(account_id.hash(), *hash);
        }
    }

    #[cfg(feature = "borsh")]
    fn key_only() -> UniversalStateInit {
        UniversalStateInit::V1(
            UniversalStateInitV1::default()
                .with_access_key(PublicKeyHandle::MLDSA65Hash([0x11; 32])),
        )
    }

    #[cfg(feature = "borsh")]
    fn contract() -> UniversalStateInit {
        UniversalStateInit::V1(
            UniversalStateInitV1::default()
                .with_code(GlobalContractId::CodeHash([0x22; 32]))
                .with_data_entry(b"key", b"value"),
        )
    }

    /// Borsh bytes pinned to nearcore's `UniversalStateInit` encoding (`near-primitives`
    /// `universal_state_init.rs`).
    #[test]
    #[cfg(feature = "borsh")]
    fn borsh_encoding_matches_nearcore() {
        // V1 | code: None | data: 0 entries | access_keys: 1 x (tag 3, 32 bytes)
        let mut expected = vec![0u8, 0, 0, 0, 0, 0, 1, 0, 0, 0, 3];
        expected.extend_from_slice(&[0x11; 32]);
        assert_eq!(key_only().to_raw().0, expected);

        // V1 | code: Some(CodeHash) | data: 1 entry | access_keys: 0
        let mut expected = vec![0u8, 1, 0];
        expected.extend_from_slice(&[0x22; 32]);
        expected.extend_from_slice(&[1, 0, 0, 0, 3, 0, 0, 0]);
        expected.extend_from_slice(b"key");
        expected.extend_from_slice(&[5, 0, 0, 0]);
        expected.extend_from_slice(b"value");
        expected.extend_from_slice(&[0, 0, 0, 0]);
        assert_eq!(contract().to_raw().0, expected);

        for state_init in [key_only(), contract()] {
            let decoded = UniversalStateInit::from_raw(&state_init.to_raw()).unwrap();
            assert_eq!(decoded, state_init);
        }
    }

    #[test]
    #[cfg(feature = "borsh")]
    fn from_raw_rejects_trailing_and_truncated_bytes() {
        let bytes = key_only().to_raw().0;
        let mut trailing = bytes.clone();
        trailing.push(0);
        assert!(UniversalStateInit::from_raw(&trailing.into()).is_err());
        assert!(UniversalStateInit::from_raw(&bytes[..bytes.len() - 1].to_vec().into()).is_err());
        // An unknown version tag is not a state init this crate can type.
        assert!(UniversalStateInit::from_raw(&vec![1].into()).is_err());
    }

    /// Account ids pinned to nearcore's `test_derive_universal_account_id` vectors.
    #[test]
    #[cfg(feature = "borsh")]
    fn derive_account_id_matches_nearcore() {
        assert_eq!(
            key_only().derive_account_id().as_str(),
            "0ux8te7g99f9kqzdtp9h4qnwt9aczpgayymmtbdc50w199rcw3at1g"
        );
        assert_eq!(
            contract().derive_account_id().as_str(),
            "0uzvdgbyea2rd8ywx0kw3cg4vc0ez1x5fc2gyks4fdz9ae0xxvzan0"
        );
    }

    #[test]
    #[cfg(feature = "borsh")]
    fn access_keys_are_ordered_by_tag_then_bytes() {
        let state_init = UniversalStateInitV1::default()
            .with_access_key(PublicKeyHandle::MLDSA65Hash([0x00; 32]))
            .with_access_key(PublicKeyHandle::SECP256K1([0x00; 64]))
            .with_access_key(PublicKeyHandle::ED25519([0xff; 32]))
            .with_access_key(PublicKeyHandle::ED25519([0x00; 32]));
        let tags: Vec<u8> =
            state_init.access_keys.iter().map(|k| borsh::to_vec(k).unwrap()[0]).collect();
        assert_eq!(tags, [0, 0, 1, 3]);
        let first_two: Vec<&[u8]> =
            state_init.access_keys.iter().take(2).map(|k| k.key_data()).collect();
        assert_eq!(first_two, [&[0x00; 32][..], &[0xff; 32][..]]);
    }

    /// Like nearcore's `RawStateInit`: a length-prefixed byte vector in borsh, a base64 string
    /// in JSON, and the id hashes the bytes without either wrapper.
    #[test]
    #[cfg(all(feature = "serde", feature = "borsh"))]
    fn raw_state_init_wire_forms() {
        let raw = key_only().to_raw();
        let mut expected = (raw.0.len() as u32).to_le_bytes().to_vec();
        expected.extend_from_slice(&raw.0);
        assert_eq!(borsh::to_vec(&raw).unwrap(), expected);
        assert_eq!(borsh::from_slice::<RawStateInit>(&expected).unwrap(), raw);

        let json = serde_json::to_value(RawStateInit(b"key".to_vec())).unwrap();
        assert_eq!(json, serde_json::json!("a2V5"));
        assert_eq!(
            serde_json::from_value::<RawStateInit>(json).unwrap(),
            RawStateInit(b"key".to_vec())
        );

        assert_eq!(raw.derive_account_id(), key_only().derive_account_id());
        assert_eq!(raw.derive_account_id(), derive_universal_account_id(&raw.0));
        assert_ne!(raw.derive_account_id(), derive_universal_account_id(expected));
    }

    #[test]
    #[cfg(feature = "schemars-v0_8")]
    fn typed_data_json_schema_is_base64() {
        let schema = serde_json::to_value(schemars_v0_8::schema_for!(UniversalStateInit)).unwrap();
        let data = &schema["definitions"]["UniversalStateInitV1"]["properties"]["data"];
        assert_eq!(data["type"], "object");
        assert_eq!(data["additionalProperties"]["type"], "string");
        assert_eq!(data["additionalProperties"]["contentEncoding"], "base64");
    }

    #[test]
    #[cfg(feature = "schemars-v0_8")]
    fn raw_state_init_json_schema_is_base64_string() {
        let schema = serde_json::to_value(schemars_v0_8::schema_for!(RawStateInit)).unwrap();
        assert_eq!(schema["type"], "string");
        assert_eq!(schema["contentEncoding"], "base64");
    }

    #[test]
    #[cfg(feature = "serde")]
    fn json_shape() {
        let state_init = UniversalStateInit::V1(
            UniversalStateInitV1::default()
                .with_code(GlobalContractId::AccountId("code.near".parse().unwrap()))
                .with_data_entry(b"key", b"value")
                .with_access_key(PublicKeyHandle::ED25519([0; 32])),
        );
        let json = serde_json::to_value(&state_init).unwrap();
        assert_eq!(
            json,
            serde_json::json!({
                "v1": {
                    "code": { "account_id": "code.near" },
                    "data": { "a2V5": "dmFsdWU=" },
                    "access_keys": ["ed25519:11111111111111111111111111111111"],
                }
            })
        );
        assert_eq!(serde_json::from_value::<UniversalStateInit>(json).unwrap(), state_init);
        assert_eq!(
            serde_json::from_value::<UniversalStateInit>(serde_json::json!({ "v1": {} })).unwrap(),
            UniversalStateInit::V1(UniversalStateInitV1::default())
        );
    }
}
