//! Universal account (`0u`) state initialization types.
//!
//! A universal account is a post-quantum-safe successor to implicit and deterministic accounts:
//! its address is `0u` followed by the Crockford base32 encoding of the SHA3-256 hash of the
//! borsh-serialized [`UniversalStateInit`] that creates it. The id therefore commits to the
//! *exact bytes* that are hashed, so the types here always emit the canonical encoding (sorted
//! `BTree*` containers, one struct per version), which matches what nearcore emits for the same
//! logical value.
//!
//! Universal accounts are created with [`Promise::universal_state_init`](crate::Promise::universal_state_init)
//! (or the lower-level [`env::promise_batch_action_universal_state_init`]),
//! and their id can be computed ahead of time with [`UniversalStateInit::derive_account_id`] or
//! the [`env::universal_state_init_to_account_id`]
//! host function.
//!
//! # Requirements
//!
//! Requires the host to support universal accounts (nearcore protocol version 87+, shipped in
//! nearcore 2.14).

use std::collections::{BTreeMap, BTreeSet};
use std::str::FromStr;

use near_sdk_macros::near;

use crate::{AccountId, CryptoHash, CurveType, GlobalContractId, PublicKey, env};

/// Domain-separation tag nearcore prepends to a full ML-DSA-65 public key before hashing it
/// into its on-trie handle.
const ML_DSA_65_PUBKEY_HASH_DOMAIN: &[u8] = b"near:ml-dsa-65-pubkey-hash:v1";

/// Length of a full ML-DSA-65 public key.
const ML_DSA_65_PUBLIC_KEY_LENGTH: usize = 1952;

/// Compact on-trie form of an access key, as stored in a [`UniversalStateInit`].
///
/// This mirrors nearcore's `PublicKeyHandle`: the full key for `ed25519` and `secp256k1`, and the
/// 32-byte SHA3-256 hash (with a domain-separation prefix) for ML-DSA-65, whose full 1952-byte key
/// is never stored on-chain. Build one from a [`PublicKey`] with [`From`] / [`Into`]; the ML-DSA-65
/// hashing is done for you.
///
/// The borsh encoding is a single tag byte (`0` ed25519, `1` secp256k1, `3` ML-DSA-65 hash) followed
/// by the raw bytes, and the ordering is by tag, then by bytes, so a `BTreeSet<PublicKeyHandle>`
/// serializes exactly like nearcore's.
///
/// # Example
/// ```
/// use near_sdk::universal_state_init::PublicKeyHandle;
/// use near_sdk::PublicKey;
///
/// let pk: PublicKey = "ed25519:6E8sCci9badyRkXb3JoRpBj5p8C6Tw41ELDZoiihKEtp".parse().unwrap();
/// let handle = PublicKeyHandle::from(&pk);
/// assert_eq!(handle.to_string(), "ed25519:6E8sCci9badyRkXb3JoRpBj5p8C6Tw41ELDZoiihKEtp");
/// assert_eq!(handle.to_string().parse::<PublicKeyHandle>().unwrap(), handle);
/// ```
#[near(inside_nearsdk, serializers = [borsh])]
#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    serde_with::SerializeDisplay,
    serde_with::DeserializeFromStr,
)]
#[borsh(use_discriminant = true)]
#[repr(u8)]
pub enum PublicKeyHandle {
    /// Full ed25519 public key.
    ED25519([u8; 32]) = 0,
    /// Full secp256k1 public key (uncompressed, 64 bytes).
    SECP256K1([u8; 64]) = 1,
    /// SHA3-256 hash of an ML-DSA-65 public key. This is what the trie stores; the full key is
    /// carried separately in transactions.
    MLDSA65Hash([u8; 32]) = 3,
}

const ML_DSA_65_HASH_PREFIX: &str = "ml-dsa-65-hash:";

impl PublicKeyHandle {
    /// Derives the on-trie handle of a raw 1952-byte ML-DSA-65 public key, exactly as nearcore
    /// does: `SHA3-256("near:ml-dsa-65-pubkey-hash:v1" || key)`.
    pub fn from_ml_dsa_65_public_key(public_key: &[u8; ML_DSA_65_PUBLIC_KEY_LENGTH]) -> Self {
        let mut input = Vec::with_capacity(ML_DSA_65_PUBKEY_HASH_DOMAIN.len() + public_key.len());
        input.extend_from_slice(ML_DSA_65_PUBKEY_HASH_DOMAIN);
        input.extend_from_slice(public_key);
        Self::MLDSA65Hash(env::sha3_256(&input))
    }

    /// The raw bytes of the handle, without the tag byte.
    pub fn key_data(&self) -> &[u8] {
        match self {
            Self::ED25519(data) => data,
            Self::SECP256K1(data) => data,
            Self::MLDSA65Hash(data) => data,
        }
    }
}

impl From<&PublicKey> for PublicKeyHandle {
    fn from(public_key: &PublicKey) -> Self {
        let data = &public_key.as_bytes()[1..];
        match public_key.curve_type() {
            CurveType::ED25519 => Self::ED25519(data.try_into().unwrap_or_else(|_| env::abort())),
            CurveType::SECP256K1 => {
                Self::SECP256K1(data.try_into().unwrap_or_else(|_| env::abort()))
            }
            CurveType::MLDSA65 => {
                Self::from_ml_dsa_65_public_key(data.try_into().unwrap_or_else(|_| env::abort()))
            }
        }
    }
}

impl From<PublicKey> for PublicKeyHandle {
    fn from(public_key: PublicKey) -> Self {
        Self::from(&public_key)
    }
}

impl std::fmt::Display for PublicKeyHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let prefix = match self {
            Self::ED25519(_) => "ed25519:",
            Self::SECP256K1(_) => "secp256k1:",
            Self::MLDSA65Hash(_) => ML_DSA_65_HASH_PREFIX,
        };
        write!(f, "{prefix}{}", bs58::encode(self.key_data()).into_string())
    }
}

/// Error returned when a string is not a valid [`PublicKeyHandle`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsePublicKeyHandleError {
    message: &'static str,
}

impl std::fmt::Display for ParsePublicKeyHandleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.message)
    }
}

impl std::error::Error for ParsePublicKeyHandleError {}

impl FromStr for PublicKeyHandle {
    type Err = ParsePublicKeyHandleError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        fn decode<const N: usize>(data: &str) -> Result<[u8; N], ParsePublicKeyHandleError> {
            let mut buf = [0u8; N];
            match bs58::decode(data).onto(&mut buf) {
                Ok(len) if len == N => Ok(buf),
                _ => Err(ParsePublicKeyHandleError { message: "invalid public key handle data" }),
            }
        }

        if let Some(data) = value.strip_prefix(ML_DSA_65_HASH_PREFIX) {
            return decode(data).map(Self::MLDSA65Hash);
        }
        if value.starts_with("ml-dsa-65:") {
            return Err(ParsePublicKeyHandleError {
                message: "a full ml-dsa-65 public key is not a handle; convert it with \
                          `PublicKeyHandle::from(PublicKey)` or use the `ml-dsa-65-hash:` form",
            });
        }
        let (curve, data) = match value.split_once(':') {
            Some((curve, data)) => (curve.parse::<CurveType>(), data),
            // No prefix: default to ed25519, like `PublicKey`.
            None => (Ok(CurveType::ED25519), value),
        };
        match curve {
            Ok(CurveType::ED25519) => decode(data).map(Self::ED25519),
            Ok(CurveType::SECP256K1) => decode(data).map(Self::SECP256K1),
            _ => Err(ParsePublicKeyHandleError { message: "unknown public key handle curve" }),
        }
    }
}

#[cfg(feature = "abi")]
impl schemars::JsonSchema for PublicKeyHandle {
    fn is_referenceable() -> bool {
        false
    }

    fn schema_name() -> String {
        String::schema_name()
    }

    fn json_schema(r#gen: &mut schemars::r#gen::SchemaGenerator) -> schemars::schema::Schema {
        String::json_schema(r#gen)
    }
}

/// Versioned initial state of a `0u` universal account.
///
/// The discriminant is the only version marker; new fields or semantics arrive as a new variant.
/// See the [module docs](self) for how the account id is derived from it.
///
/// # Example
/// ```
/// use near_sdk::universal_state_init::{UniversalStateInit, UniversalStateInitV1};
/// use near_sdk::{GlobalContractId, PublicKey};
///
/// let pk: PublicKey = "ed25519:6E8sCci9badyRkXb3JoRpBj5p8C6Tw41ELDZoiihKEtp".parse().unwrap();
/// let state_init = UniversalStateInit::from(
///     UniversalStateInitV1::default()
///         .with_code(GlobalContractId::CodeHash([0x22; 32]))
///         .with_data_entry(b"key", b"value")
///         .with_access_key(pk),
/// );
/// let account_id = state_init.derive_account_id();
/// assert!(account_id.as_str().starts_with("0u"));
/// assert_eq!(account_id.len(), 54);
/// ```
#[near(inside_nearsdk, serializers = [json, borsh])]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[borsh(use_discriminant = true)]
#[serde(rename_all = "snake_case")]
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
#[near(inside_nearsdk, serializers = [json, borsh])]
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct UniversalStateInitV1 {
    /// Contract code, or `None` for a key-only account.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub code: Option<GlobalContractId>,
    /// Initial storage; empty unless seeded. Sorted keys give a canonical encoding.
    #[serde_as(
        as = "::std::collections::BTreeMap<::serde_with::base64::Base64, ::serde_with::base64::Base64>"
    )]
    #[cfg_attr(feature = "abi", schemars(with = "::std::collections::BTreeMap<String, String>"))]
    #[serde(default, skip_serializing_if = "::std::collections::BTreeMap::is_empty")]
    pub data: BTreeMap<Vec<u8>, Vec<u8>>,
    /// Full-access keys as compact on-trie handles. Sorted for a canonical encoding.
    #[serde(default, skip_serializing_if = "::std::collections::BTreeSet::is_empty")]
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

    /// Adds a full-access key. Accepts a [`PublicKey`] (ML-DSA-65 keys are hashed into their
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

    /// Canonical borsh encoding of this state init: the bytes the account id commits to, and
    /// what [`env::promise_batch_action_universal_state_init`]
    /// sends to the runtime.
    pub fn to_bytes(&self) -> Vec<u8> {
        borsh::to_vec(self).unwrap_or_else(|_| env::abort())
    }

    /// The `0u` account id this state init creates: the SHA3-256 hash of [`to_bytes`](Self::to_bytes),
    /// Crockford-base32 encoded.
    ///
    /// Computed in the contract from the `sha3_256` host function (or in pure Rust off-chain);
    /// the [`env::universal_state_init_to_account_id`]
    /// host function returns the same id.
    pub fn derive_account_id(&self) -> AccountId {
        derive_universal_account_id(&self.to_bytes())
    }
}

/// Derives the `0u` account id from raw state-init bytes, like nearcore's
/// `derive_universal_account_id`: SHA3-256 over exactly those bytes, Crockford-base32 encoded.
pub(crate) fn derive_universal_account_id(state_init: &[u8]) -> AccountId {
    encode_universal_account_id(&env::sha3_256(state_init))
}

/// Scheme + hash-function marker of a universal account id.
const UAID_PREFIX: &str = "0u";
/// Base32 symbols encoding the 256-bit hash (`ceil(256 / 5)`).
const UAID_DATA_SYMBOLS: usize = 52;
/// Crockford base32, lowercase, excluding `i l o u` to reduce transcription errors.
const CROCKFORD: &[u8; 32] = b"0123456789abcdefghjkmnpqrstvwxyz";

/// Encodes a 32-byte hash as a `0u` universal account id (`0u` + 52 Crockford-base32 symbols),
/// exactly like nearcore's `encode_universal_account_id`.
// TODO: replace with `near_account_id::UniversalAccountId::from_hash` once near-account-id 3.1
// ships it.
pub(crate) fn encode_universal_account_id(hash: &CryptoHash) -> AccountId {
    let mut s = String::with_capacity(UAID_PREFIX.len() + UAID_DATA_SYMBOLS);
    s.push_str(UAID_PREFIX);
    // 32 bytes -> 52 five-bit symbols, MSB-first. The 256 bits leave 1 bit in the final symbol,
    // padded on the right with 4 zero bits.
    let mut acc: u32 = 0;
    let mut nbits: u32 = 0;
    for &byte in hash {
        acc = (acc << 8) | byte as u32;
        nbits += 8;
        while nbits >= 5 {
            nbits -= 5;
            s.push(CROCKFORD[((acc >> nbits) & 0x1f) as usize] as char);
        }
        acc &= (1u32 << nbits) - 1;
    }
    s.push(CROCKFORD[((acc << (5 - nbits)) & 0x1f) as usize] as char);
    // The emitted charset and length are always a valid account id.
    s.parse().unwrap_or_else(|_| env::abort())
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

    #[test]
    fn encoder_matches_nearcore_known_answers() {
        for (hash, expected) in UAID_KATS {
            assert_eq!(encode_universal_account_id(hash).as_str(), *expected);
        }
    }

    fn key_only() -> UniversalStateInit {
        UniversalStateInit::V1(
            UniversalStateInitV1::default()
                .with_access_key(PublicKeyHandle::MLDSA65Hash([0x11; 32])),
        )
    }

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
    fn borsh_encoding_matches_nearcore() {
        // V1 | code: None | data: 0 entries | access_keys: 1 x (tag 3, 32 bytes)
        let mut expected = vec![0u8, 0, 0, 0, 0, 0, 1, 0, 0, 0, 3];
        expected.extend_from_slice(&[0x11; 32]);
        assert_eq!(key_only().to_bytes(), expected);

        // V1 | code: Some(CodeHash) | data: 1 entry | access_keys: 0
        let mut expected = vec![0u8, 1, 0];
        expected.extend_from_slice(&[0x22; 32]);
        expected.extend_from_slice(&[1, 0, 0, 0, 3, 0, 0, 0]);
        expected.extend_from_slice(b"key");
        expected.extend_from_slice(&[5, 0, 0, 0]);
        expected.extend_from_slice(b"value");
        expected.extend_from_slice(&[0, 0, 0, 0]);
        assert_eq!(contract().to_bytes(), expected);

        for state_init in [key_only(), contract()] {
            let decoded: UniversalStateInit = borsh::from_slice(&state_init.to_bytes()).unwrap();
            assert_eq!(decoded, state_init);
        }
    }

    /// Account ids pinned to nearcore's `test_derive_universal_account_id` vectors.
    #[test]
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

    #[test]
    fn public_key_handle_from_public_key() {
        let ed: PublicKey = "ed25519:6E8sCci9badyRkXb3JoRpBj5p8C6Tw41ELDZoiihKEtp".parse().unwrap();
        let handle = PublicKeyHandle::from(&ed);
        assert_eq!(handle.key_data(), &ed.as_bytes()[1..]);
        assert_eq!(borsh::to_vec(&handle).unwrap(), ed.as_bytes());
        assert_eq!(handle.to_string(), ed.to_string());

        let secp: PublicKey = "secp256k1:5r22SrjrDvgY3wdQsnjgxkeAbU1VcM71FYvALEQWihjM3Xk4Be1CpETTqFccChQr4iJwDroSDVmgaWZv2AcXvYeL".parse().unwrap();
        let handle = PublicKeyHandle::from(&secp);
        assert_eq!(borsh::to_vec(&handle).unwrap(), secp.as_bytes());
        assert_eq!(handle.to_string(), secp.to_string());

        // ML-DSA-65: the handle is the domain-separated SHA3-256 of the raw key, tag 3.
        let raw = [0x42u8; ML_DSA_65_PUBLIC_KEY_LENGTH];
        let ml_dsa = PublicKey::from_parts(CurveType::MLDSA65, raw.to_vec()).unwrap();
        let handle = PublicKeyHandle::from(&ml_dsa);
        let expected = env::sha3_256([ML_DSA_65_PUBKEY_HASH_DOMAIN, &raw[..]].concat());
        assert_eq!(handle, PublicKeyHandle::MLDSA65Hash(expected));
        let encoded = borsh::to_vec(&handle).unwrap();
        assert_eq!(encoded[0], 3);
        assert_eq!(&encoded[1..], &expected);
        assert_eq!(
            handle.to_string(),
            format!("ml-dsa-65-hash:{}", bs58::encode(expected).into_string())
        );
    }

    #[test]
    fn public_key_handle_parse_roundtrip() {
        for handle in [
            PublicKeyHandle::ED25519([7; 32]),
            PublicKeyHandle::SECP256K1([8; 64]),
            PublicKeyHandle::MLDSA65Hash([9; 32]),
        ] {
            assert_eq!(handle.to_string().parse::<PublicKeyHandle>().unwrap(), handle);
            let json = serde_json::to_string(&handle).unwrap();
            assert_eq!(json, format!("\"{handle}\""));
            assert_eq!(serde_json::from_str::<PublicKeyHandle>(&json).unwrap(), handle);
        }
        assert!("ml-dsa-65:abc".parse::<PublicKeyHandle>().is_err());
        assert!("ml-dsa-65-hash:abc".parse::<PublicKeyHandle>().is_err());
        assert!("rsa:abc".parse::<PublicKeyHandle>().is_err());
    }

    #[test]
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
