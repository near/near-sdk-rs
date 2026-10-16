use std::collections::{BTreeMap, BTreeSet};

use near_sdk_core::types::PublicKeyHandle;

use crate::GlobalContractId;

#[cfg(feature = "serde")]
use serde_with::base64::Base64;

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
    /// Checks that these bytes decode to a supported state init and are its canonical encoding.
    ///
    /// Storage keys must be strictly increasing in lexicographic byte order. Access-key handles
    /// must be strictly increasing by tag, then handle bytes. Duplicates, trailing bytes,
    /// malformed encodings and versions unknown to this crate are rejected. Storage values are
    /// opaque bytes; their contents are not normalized or validated.
    ///
    /// This is the check nearcore applies from protocol version 88 on (`from_raw_strict`), and
    /// the errors match the statuses `universal_state_init_to_account_id` reports. It is local,
    /// though: it knows only this crate's state-init versions, and it does not establish that
    /// initialization will succeed on a particular network (limits, code availability and
    /// funding still apply). Contracts forwarding future versions should ask the host.
    ///
    /// Requires the `borsh` feature.
    #[cfg(feature = "borsh")]
    pub fn validate_canonical(&self) -> Result<(), StateInitError> {
        let decoded = UniversalStateInit::try_from(self).map_err(|_| StateInitError::Malformed)?;
        if decoded.to_raw() != *self {
            return Err(StateInitError::NonCanonical);
        }
        Ok(())
    }

    /// Validates deserialization and canonicity, then derives the `0u` account id from the
    /// original bytes using SHA3-256 and Crockford base32.
    ///
    /// Use this before displaying an address for funding. Errors leave the original bytes
    /// unchanged; they are never normalized into a different account. See
    /// [`validate_canonical`](Self::validate_canonical) for the scope of validation.
    ///
    /// Requires the `borsh` feature.
    #[cfg(feature = "borsh")]
    pub fn try_derive_account_id(&self) -> Result<near_account_id::AccountId, StateInitError> {
        self.validate_canonical()?;
        Ok(self.hash_account_id())
    }

    /// SHA3-256 over exactly these bytes, encoded as a `0u` id. Callers validate first.
    ///
    /// [`near_digest::sha3::Sha3_256`] is the `sha3_256` host function in contract builds
    /// (`--cfg near`, set by `cargo-near`) and pure Rust elsewhere, with identical output.
    #[cfg(feature = "borsh")]
    fn hash_account_id(&self) -> near_account_id::AccountId {
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

/// Why bytes are not a valid [`RawStateInit`] payload.
///
/// Mirrors nearcore's `near_primitives_core::universal_state_init::StateInitError`. The host
/// function `universal_state_init_to_account_id` reports these as exit statuses `1` and `2`, and
/// [`RawStateInit::validate_canonical`] returns them for the versions this crate knows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StateInitError {
    /// The bytes do not decode into a `UniversalStateInit`: malformed, truncated, trailing
    /// bytes, or a version the decoder does not know.
    Malformed,
    /// The bytes decode, but are not the canonical borsh encoding of the result (unsorted or
    /// duplicate storage keys or access-key handles).
    NonCanonical,
}

impl core::fmt::Display for StateInitError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(match self {
            Self::Malformed => "bytes do not decode into a universal state init",
            Self::NonCanonical => "state init is not in canonical borsh form",
        })
    }
}

impl std::error::Error for StateInitError {}

/// Decodes raw state-init bytes, accepting any well-formed encoding.
///
/// Trailing bytes, malformed bytes and unknown versions are rejected. This is an inspection
/// conversion, not a canonicality check: decoding can normalize non-canonical collections.
/// Use [`RawStateInit::validate_canonical`] or [`RawStateInit::try_derive_account_id`] to reject
/// such input, and keep the original [`RawStateInit`] for forwarding. nearcore's lenient
/// counterpart is `UniversalStateInit::from_raw_lenient`.
///
/// Collections are decoded leniently even if a crate in the dependency graph enables Borsh's
/// `de_strict_order` feature, so the result does not depend on feature unification.
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

    use near_primitives_core::universal_state_init::StateInitError as NearcoreStateInitError;

    impl From<NearcoreStateInitError> for StateInitError {
        #[inline]
        fn from(error: NearcoreStateInitError) -> Self {
            match error {
                NearcoreStateInitError::Malformed => Self::Malformed,
                NearcoreStateInitError::NonCanonical => Self::NonCanonical,
            }
        }
    }

    impl From<StateInitError> for NearcoreStateInitError {
        #[inline]
        fn from(error: StateInitError) -> Self {
            match error {
                StateInitError::Malformed => Self::Malformed,
                StateInitError::NonCanonical => Self::NonCanonical,
            }
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
#[cfg_attr(feature = "borsh", derive(borsh::BorshSerialize))]
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
    #[cfg_attr(feature = "serde", serde_as(as = "BTreeMap<Base64, Base64>"))]
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "::std::collections::BTreeMap::is_empty")
    )]
    pub data: BTreeMap<Vec<u8>, Vec<u8>>,
    /// Full-access keys as compact on-trie handles. Sorted for a canonical encoding.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "::std::collections::BTreeSet::is_empty")
    )]
    pub access_keys: BTreeSet<PublicKeyHandle>,
}

/// Decoded by hand so the result does not depend on whether Borsh's `de_strict_order` feature is
/// enabled anywhere in the dependency graph: unsorted or duplicate entries always decode, and
/// [`RawStateInit::validate_canonical`] reports them as [`StateInitError::NonCanonical`], as the
/// host does. A later duplicate replaces the earlier entry.
#[cfg(feature = "borsh")]
impl borsh::BorshDeserialize for UniversalStateInitV1 {
    fn deserialize_reader<R: borsh::io::Read>(reader: &mut R) -> borsh::io::Result<Self> {
        let code = Option::<GlobalContractId>::deserialize_reader(reader)?;
        let mut data = BTreeMap::new();
        for _ in 0..u32::deserialize_reader(reader)? {
            let key = Vec::<u8>::deserialize_reader(reader)?;
            data.insert(key, Vec::<u8>::deserialize_reader(reader)?);
        }
        let mut access_keys = BTreeSet::new();
        for _ in 0..u32::deserialize_reader(reader)? {
            access_keys.insert(PublicKeyHandle::deserialize_reader(reader)?);
        }
        Ok(Self { code, data, access_keys })
    }
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

    /// The `0u` account id of this state init's canonical encoding.
    ///
    /// The id commits to the bytes, not the logical value. A contract that forwards a state init
    /// it was handed must validate and derive from those bytes
    /// ([`RawStateInit::try_derive_account_id`]) and forward them unchanged, never decode and
    /// re-encode.
    ///
    /// # Availability
    ///
    /// Requires the `borsh` feature. Uses the SHA3-256 host function with `--cfg near`, pure
    /// Rust hashing otherwise. No decoding is needed because typed encoding is canonical.
    #[cfg(feature = "borsh")]
    pub fn derive_account_id(&self) -> near_account_id::AccountId {
        self.to_raw().hash_account_id()
    }
}

/// `0u` followed by the 32-byte hash in lowercase Crockford base32: 52 symbols, most significant
/// bit first, 5 bits per symbol, the last symbol carrying one data bit and four zero bits. Same as
/// nearcore's 0u encoder.
#[cfg(feature = "borsh")]
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
    #[cfg(feature = "borsh")]
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
    #[cfg(feature = "borsh")]
    fn encoder_matches_nearcore_known_answers() {
        for (hash, expected) in UAID_KATS {
            let account_id = universal_account_id_from_hash(hash);
            assert_eq!(account_id.as_str(), *expected);
            assert_eq!(account_id.len(), 54);
            assert_eq!(account_id, expected.parse::<AccountId>().unwrap());
            assert_eq!(
                account_id.get_account_type(),
                near_account_id::AccountType::UniversalAccount
            );
        }
    }

    /// Differential check against nearcore's encoder over 2048 pseudo-random hashes.
    #[test]
    #[cfg(all(feature = "borsh", feature = "near-primitives-interop"))]
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
