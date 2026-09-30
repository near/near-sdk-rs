use std::collections::{BTreeMap, BTreeSet};

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
/// Build one from a typed value with `UniversalStateInit::to_raw` or `From`, or wrap bytes you were
/// handed with `RawStateInit::from(bytes)`. Decode one with `UniversalStateInit::try_from` (the
/// typed conversions need the `borsh` feature).
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
    /// encoded, the same derivation nearcore applies to a `UniversalStateInit` action.
    ///
    /// SHA3-256 comes from [`near_digest::sha3::Sha3_256`]: the `sha3_256` host function in
    /// contract builds (`--cfg near`, set by `cargo-near`) and pure Rust elsewhere, with identical
    /// output.
    pub fn derive_account_id(&self) -> near_account_id::AccountId {
        use near_digest::Digest;

        let hash: [u8; 32] = near_digest::sha3::Sha3_256::digest(&self.0).into();
        universal_account_id_from_hash(&hash)
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

/// Decodes raw state-init bytes, like nearcore's `UniversalStateInit::from_raw`.
///
/// Trailing bytes, malformed bytes and unknown versions are rejected. A non-canonical encoding
/// decodes fine, but the account id commits to the original bytes, so re-encoding the result can
/// give a different id. Keep the [`RawStateInit`] to forward it or to derive its id.
///
/// Decoding can be stricter than the chain: borsh's `de_strict_order` cargo feature, if any crate
/// in the dependency graph enables it, makes this reject unsorted or duplicate keys that nearcore
/// accepts. Account ids are unaffected, since they never go through decoding.
///
/// Requires the `borsh` feature.
#[cfg(feature = "borsh")]
impl TryFrom<&RawStateInit> for UniversalStateInit {
    type Error = std::io::Error;

    fn try_from(raw: &RawStateInit) -> Result<Self, Self::Error> {
        borsh::from_slice(&raw.0)
    }
}

/// Decodes raw state-init bytes; see the `TryFrom<&RawStateInit>` impl.
///
/// Requires the `borsh` feature.
#[cfg(feature = "borsh")]
impl TryFrom<RawStateInit> for UniversalStateInit {
    type Error = std::io::Error;

    #[inline]
    fn try_from(raw: RawStateInit) -> Result<Self, Self::Error> {
        Self::try_from(&raw)
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
/// There is deliberately no serde/JSON form: JSON callers (contract arguments, RPC) carry the
/// [`RawStateInit`] as base64, like nearcore's `ActionView`. The id commits to the bytes, so
/// anything forwarding a state init it did not build must carry the bytes, not a typed value.
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
    feature = "borsh",
    derive(borsh::BorshSerialize, borsh::BorshDeserialize),
    borsh(use_discriminant = true)
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
#[cfg_attr(feature = "borsh", derive(borsh::BorshSerialize, borsh::BorshDeserialize))]
#[cfg_attr(feature = "abi", derive(borsh::BorshSchema))]
pub struct UniversalStateInitV1 {
    /// Contract code, or `None` for a key-only account.
    pub code: Option<GlobalContractId>,
    /// Initial storage; empty unless seeded. Sorted keys give a canonical encoding.
    pub data: BTreeMap<Vec<u8>, Vec<u8>>,
    /// Full-access keys as compact on-trie handles. Sorted for a canonical encoding.
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

    /// The `0u` account id of this state init's canonical encoding. Shorthand for
    /// `self.to_raw().derive_account_id()`.
    ///
    /// The id commits to the bytes, not the logical value. A contract that forwards a state init
    /// it was handed must derive the id from those bytes ([`RawStateInit::derive_account_id`]) and
    /// forward them unchanged, never decode and re-encode.
    ///
    /// # Availability
    ///
    /// Requires the `borsh` feature. See [`RawStateInit::derive_account_id`] for the SHA3-256
    /// backend.
    #[cfg(feature = "borsh")]
    pub fn derive_account_id(&self) -> near_account_id::AccountId {
        self.to_raw().derive_account_id()
    }
}

/// `0u` followed by the 32-byte hash in lowercase Crockford base32: 52 symbols, most significant
/// bit first, 5 bits per symbol, the last symbol carrying one data bit and four zero bits. Same as
/// nearcore's 0u encoder.
fn universal_account_id_from_hash(hash: &[u8; 32]) -> near_account_id::AccountId {
    const ALPHABET: &[u8; 32] = b"0123456789abcdefghjkmnpqrstvwxyz";

    let mut id = String::with_capacity(54);
    id.push_str("0u");
    let (mut buffer, mut bits) = (0u32, 0u32);
    for &byte in hash {
        buffer = (buffer << 8) | u32::from(byte);
        bits += 8;
        while bits >= 5 {
            bits -= 5;
            id.push(char::from(ALPHABET[((buffer >> bits) & 0x1f) as usize]));
        }
        buffer &= (1 << bits) - 1;
    }
    // 256 bits leave 1 over: pad it with 4 zero bits into a final symbol.
    id.push(char::from(ALPHABET[((buffer << (5 - bits)) & 0x1f) as usize]));

    id.parse().expect("`0u` + 52 lowercase base32 symbols is a valid account id")
}

#[cfg(test)]
mod tests {
    use super::*;
    use near_account_id::AccountId;

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

    /// The id is consensus-critical, so the encoder is pinned to nearcore's own vectors.
    #[test]
    fn encoder_matches_nearcore_known_answers() {
        for (hash, expected) in UAID_KATS {
            let account_id = universal_account_id_from_hash(hash);
            assert_eq!(account_id.as_str(), *expected);
            assert_eq!(account_id.len(), 54);
            assert_eq!(account_id, expected.parse::<AccountId>().unwrap());
        }
    }

    /// Differential check against nearcore's encoder over 2048 pseudo-random hashes.
    #[test]
    #[cfg(feature = "near-primitives-interop")]
    fn encoder_matches_nearcore_encoder() {
        use near_digest::Digest;

        for i in 0u32..2048 {
            let hash: [u8; 32] = near_digest::sha3::Sha3_256::digest(i.to_le_bytes()).into();
            assert_eq!(
                universal_account_id_from_hash(&hash).as_str(),
                near_primitives_core::universal_account_id::encode_universal_account_id(&hash)
                    .as_str()
            );
        }
    }
}
