use bs58::decode::Error as B58Error;
use std::convert::TryFrom;

/// PublicKey curve
#[derive(Debug, Clone, Copy, PartialOrd, Ord, Eq, PartialEq)]
#[cfg_attr(
    feature = "borsh",
    derive(borsh::BorshSerialize, borsh::BorshDeserialize),
    borsh(use_discriminant = true)
)]
#[cfg_attr(feature = "abi", derive(borsh::BorshSchema))]
#[repr(u8)]
pub enum CurveType {
    ED25519 = 0,
    SECP256K1 = 1,
    /// FIPS 204 ML-DSA-65 post-quantum signature scheme.
    ///
    /// Added in nearcore 2.13 (protocol v85) as a third access-key/signature
    /// scheme alongside ed25519 and secp256k1. Only the full public-key form is
    /// represented here (1952 raw bytes); the runtime-internal SHA3-256 trie hash
    /// handle form (`ml-dsa-65-hash:`) is never exposed to contracts.
    MLDSA65 = 2,
}

impl CurveType {
    fn from_u8(val: u8) -> Result<Self, ParsePublicKeyError> {
        match val {
            0 => Ok(CurveType::ED25519),
            1 => Ok(CurveType::SECP256K1),
            2 => Ok(CurveType::MLDSA65),
            _ => Err(ParsePublicKeyError { kind: ParsePublicKeyErrorKind::UnknownCurve }),
        }
    }

    /// Get the length of bytes associated to this CurveType
    const fn data_len(&self) -> usize {
        match self {
            CurveType::ED25519 => 32,
            CurveType::SECP256K1 => 64,
            CurveType::MLDSA65 => 1952,
        }
    }
}

impl std::fmt::Display for CurveType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CurveType::ED25519 => write!(f, "ed25519"),
            CurveType::SECP256K1 => write!(f, "secp256k1"),
            CurveType::MLDSA65 => write!(f, "ml-dsa-65"),
        }
    }
}

impl std::str::FromStr for CurveType {
    type Err = ParsePublicKeyError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        if value.eq_ignore_ascii_case("ed25519") {
            Ok(CurveType::ED25519)
        } else if value.eq_ignore_ascii_case("secp256k1") {
            Ok(CurveType::SECP256K1)
        } else if value.eq_ignore_ascii_case("ml-dsa-65") {
            Ok(CurveType::MLDSA65)
        } else {
            Err(ParsePublicKeyError { kind: ParsePublicKeyErrorKind::UnknownCurve })
        }
    }
}

#[cfg(all(not(target_arch = "wasm32"), feature = "near-crypto-interop"))]
const _: () = {
    impl TryFrom<PublicKey> for near_crypto::PublicKey {
        type Error = ParsePublicKeyError;

        fn try_from(public_key: PublicKey) -> Result<Self, Self::Error> {
            let curve_type = CurveType::from_u8(public_key.data[0])?;
            let expected_len = curve_type.data_len();

            let key_bytes = public_key.into_bytes();
            if key_bytes.len() != expected_len + 1 {
                return Err(ParsePublicKeyError {
                    kind: ParsePublicKeyErrorKind::InvalidLength(key_bytes.len()),
                });
            }

            let data = &key_bytes.as_slice()[1..];
            match curve_type {
                CurveType::ED25519 => {
                    let public_key = near_crypto::PublicKey::ED25519(
                        near_crypto::ED25519PublicKey::try_from(data).unwrap(),
                    );
                    Ok(public_key)
                }
                CurveType::SECP256K1 => {
                    let public_key = near_crypto::PublicKey::SECP256K1(
                        near_crypto::Secp256K1PublicKey::try_from(data).unwrap(),
                    );
                    Ok(public_key)
                }
                // TODO(post-quantum): wire up once near-crypto exposes MLDSA65
                CurveType::MLDSA65 => Err(ParsePublicKeyError {
                    kind: ParsePublicKeyErrorKind::Unsupported(
                        "ML-DSA-65 <-> near_crypto interop not supported until near-crypto adds the variant (nearcore 2.13)",
                    ),
                }),
            }
        }
    }

    impl From<near_crypto::PublicKey> for PublicKey {
        fn from(value: near_crypto::PublicKey) -> Self {
            let curve_type = match value {
                near_crypto::PublicKey::ED25519(_) => CurveType::ED25519,
                near_crypto::PublicKey::SECP256K1(_) => CurveType::SECP256K1,
            };
            Self { data: [[curve_type as u8].as_slice(), value.key_data()].concat() }
        }
    }
};

/// Public key in a binary format with base58 string serialization with human-readable curve.
/// The key types currently supported are `ed25519`, `secp256k1`, and `ml-dsa-65`.
///
/// Ed25519 public keys accepted are 32 bytes, secp256k1 keys are the uncompressed 64 format,
/// and ml-dsa-65 (FIPS 204 ML-DSA-65, added in nearcore 2.13) keys are 1952 bytes.
///
/// # Example
/// ```
/// use near_sdk_core::types::PublicKey;
///
/// // Compressed ed25519 key
/// let ed: PublicKey = "ed25519:6E8sCci9badyRkXb3JoRpBj5p8C6Tw41ELDZoiihKEtp".parse()
///             .unwrap();
///
/// // Uncompressed secp256k1 key
/// let secp256k1: PublicKey = "secp256k1:5r22SrjrDvgY3wdQsnjgxkeAbU1VcM71FYvALEQWihjM3Xk4Be1CpETTqFccChQr4iJwDroSDVmgaWZv2AcXvYeL"
///             .parse()
///             .unwrap();
/// ```
#[derive(Debug, Clone, PartialEq, PartialOrd, Ord, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde_with::SerializeDisplay, serde_with::DeserializeFromStr))]
#[cfg_attr(feature = "abi", derive(borsh::BorshSchema))]
pub struct PublicKey {
    data: Vec<u8>,
}

impl PublicKey {
    fn split_key_type_data(value: &str) -> Result<(CurveType, &str), ParsePublicKeyError> {
        if let Some(idx) = value.find(':') {
            let (prefix, key_data) = value.split_at(idx);
            Ok((prefix.parse::<CurveType>()?, &key_data[1..]))
        } else {
            // If there is no Default is ED25519.
            Ok((CurveType::ED25519, value))
        }
    }

    pub fn from_parts(curve: CurveType, data: Vec<u8>) -> Result<Self, ParsePublicKeyError> {
        let expected_length = curve.data_len();
        if data.len() != expected_length {
            return Err(ParsePublicKeyError {
                kind: ParsePublicKeyErrorKind::InvalidLength(data.len()),
            });
        }
        let mut bytes = Vec::with_capacity(1 + expected_length);
        bytes.push(curve as u8);
        bytes.extend(data);

        Ok(Self { data: bytes })
    }

    /// Returns a byte slice of this `PublicKey`'s contents.
    pub fn as_bytes(&self) -> &[u8] {
        &self.data
    }

    /// Converts a `PublicKey` into a byte vector.
    pub fn into_bytes(self) -> Vec<u8> {
        self.data
    }

    /// Get info about the CurveType for this public key
    pub fn curve_type(&self) -> CurveType {
        CurveType::from_u8(self.data[0]).unwrap_or_else(|_| {
            #[cfg(any(near, feature = "__near-sdk-unit-testing"))]
            {
                near_sdk_env::abort()
            }
            #[cfg(not(any(near, feature = "__near-sdk-unit-testing")))]
            {
                panic!()
            }
        })
    }
}

impl From<PublicKey> for Vec<u8> {
    fn from(v: PublicKey) -> Vec<u8> {
        v.data
    }
}

impl TryFrom<Vec<u8>> for PublicKey {
    type Error = ParsePublicKeyError;

    fn try_from(data: Vec<u8>) -> Result<Self, Self::Error> {
        if data.is_empty() {
            return Err(ParsePublicKeyError {
                kind: ParsePublicKeyErrorKind::InvalidLength(data.len()),
            });
        }

        let curve = CurveType::from_u8(data[0])?;
        if data.len() != curve.data_len() + 1 {
            return Err(ParsePublicKeyError {
                kind: ParsePublicKeyErrorKind::InvalidLength(data.len()),
            });
        }
        Ok(Self { data })
    }
}

#[cfg(feature = "borsh")]
impl borsh::BorshSerialize for PublicKey {
    fn serialize<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()> {
        borsh::BorshSerialize::serialize(&self.data, writer)
    }
}

#[cfg(feature = "borsh")]
impl borsh::BorshDeserialize for PublicKey {
    fn deserialize_reader<R: std::io::Read>(reader: &mut R) -> std::io::Result<Self> {
        <Vec<u8> as borsh::BorshDeserialize>::deserialize_reader(reader).and_then(|s| {
            Self::try_from(s).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
        })
    }
}

#[cfg(feature = "schemars-v0_8")]
impl schemars_v0_8::JsonSchema for PublicKey {
    fn is_referenceable() -> bool {
        false
    }

    fn schema_name() -> String {
        String::schema_name()
    }

    fn json_schema(
        r#gen: &mut schemars_v0_8::r#gen::SchemaGenerator,
    ) -> schemars_v0_8::schema::Schema {
        String::json_schema(r#gen)
    }
}

impl std::fmt::Display for PublicKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.curve_type() {
            CurveType::ED25519 => {
                write!(f, "ed25519:{}", bs58::encode(&self.data[1..]).into_string())
            }
            CurveType::SECP256K1 => {
                write!(f, "secp256k1:{}", bs58::encode(&self.data[1..]).into_string())
            }
            CurveType::MLDSA65 => {
                write!(f, "ml-dsa-65:{}", bs58::encode(&self.data[1..]).into_string())
            }
        }
    }
}

impl From<&PublicKey> for String {
    fn from(public_key: &PublicKey) -> Self {
        public_key.to_string()
    }
}

impl std::str::FromStr for PublicKey {
    type Err = ParsePublicKeyError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let (curve, key_data) = PublicKey::split_key_type_data(value)?;
        let data = bs58::decode(key_data).into_vec()?;
        Self::from_parts(curve, data)
    }
}
#[derive(Debug)]
pub struct ParsePublicKeyError {
    kind: ParsePublicKeyErrorKind,
}

#[derive(Debug)]
enum ParsePublicKeyErrorKind {
    InvalidLength(usize),
    Base58(B58Error),
    UnknownCurve,
    /// A curve is known to the SDK but unsupported by the operation being
    /// performed (e.g. ML-DSA-65 conversion to `near_crypto`, which is gated
    /// until near-crypto exposes the variant).
    #[cfg(all(not(target_arch = "wasm32"), feature = "near-crypto-interop"))]
    Unsupported(&'static str),
}

impl std::fmt::Display for ParsePublicKeyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.kind {
            ParsePublicKeyErrorKind::InvalidLength(l) => {
                write!(
                    f,
                    "invalid length of the public key: got {l} bytes \
                     (expected 32 for ed25519, 64 for secp256k1, 1952 for ml-dsa-65)"
                )
            }
            ParsePublicKeyErrorKind::Base58(e) => write!(f, "base58 decoding error: {e}"),
            ParsePublicKeyErrorKind::UnknownCurve => write!(f, "unknown curve kind"),
            #[cfg(all(not(target_arch = "wasm32"), feature = "near-crypto-interop"))]
            ParsePublicKeyErrorKind::Unsupported(msg) => write!(f, "{msg}"),
        }
    }
}

impl From<B58Error> for ParsePublicKeyError {
    fn from(e: B58Error) -> Self {
        Self { kind: ParsePublicKeyErrorKind::Base58(e) }
    }
}

impl std::error::Error for ParsePublicKeyError {}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(feature = "borsh")]
    use crate::borsh::{BorshDeserialize, BorshSerialize};
    use std::convert::TryInto;
    use std::str::FromStr;

    fn expected_key() -> PublicKey {
        let mut key = vec![CurveType::ED25519 as u8];
        key.extend(
            bs58::decode("6E8sCci9badyRkXb3JoRpBj5p8C6Tw41ELDZoiihKEtp").into_vec().unwrap(),
        );
        key.try_into().unwrap()
    }

    #[cfg(feature = "serde")]
    #[test]
    fn test_public_key_deser() {
        let key: PublicKey =
            serde_json::from_str("\"ed25519:6E8sCci9badyRkXb3JoRpBj5p8C6Tw41ELDZoiihKEtp\"")
                .unwrap();
        assert_eq!(key, expected_key());
    }

    #[cfg(feature = "serde")]
    #[test]
    fn test_public_key_ser() {
        let key: PublicKey = expected_key();
        let actual: String = serde_json::to_string(&key).unwrap();
        assert_eq!(actual, "\"ed25519:6E8sCci9badyRkXb3JoRpBj5p8C6Tw41ELDZoiihKEtp\"");
    }

    #[test]
    fn test_public_key_from_str() {
        let key =
            PublicKey::from_str("ed25519:6E8sCci9badyRkXb3JoRpBj5p8C6Tw41ELDZoiihKEtp").unwrap();
        assert_eq!(key, expected_key());
    }

    #[test]
    fn test_public_key_to_string() {
        let key: PublicKey = expected_key();
        let actual: String = String::from(&key);
        assert_eq!(actual, "ed25519:6E8sCci9badyRkXb3JoRpBj5p8C6Tw41ELDZoiihKEtp");
    }

    #[test]
    fn test_public_key_display() {
        let key: PublicKey = expected_key();
        // Display and From<&PublicKey> for String should produce the same output
        let display_output = format!("{}", key);
        let from_output = String::from(&key);
        assert_eq!(display_output, from_output);
        assert_eq!(display_output, "ed25519:6E8sCci9badyRkXb3JoRpBj5p8C6Tw41ELDZoiihKEtp");
    }

    #[test]
    fn test_public_key_display_secp256k1() {
        let key: PublicKey = "secp256k1:5r22SrjrDvgY3wdQsnjgxkeAbU1VcM71FYvALEQWihjM3Xk4Be1CpETTqFccChQr4iJwDroSDVmgaWZv2AcXvYeL".parse().unwrap();
        let display_output = format!("{}", key);
        assert!(display_output.starts_with("secp256k1:"));
    }

    #[test]
    fn test_curve_type_display() {
        assert_eq!(CurveType::ED25519.to_string(), "ed25519");
        assert_eq!(CurveType::SECP256K1.to_string(), "secp256k1");
        assert_eq!(CurveType::MLDSA65.to_string(), "ml-dsa-65");
    }

    #[test]
    fn test_curve_type_display_roundtrip() {
        // Display -> FromStr should roundtrip
        let ed = CurveType::ED25519;
        let parsed: CurveType = ed.to_string().parse().unwrap();
        assert_eq!(ed, parsed);

        let secp = CurveType::SECP256K1;
        let parsed: CurveType = secp.to_string().parse().unwrap();
        assert_eq!(secp, parsed);

        let mldsa = CurveType::MLDSA65;
        let parsed: CurveType = mldsa.to_string().parse().unwrap();
        assert_eq!(mldsa, parsed);
    }

    #[test]
    fn test_curve_type_from_u8_mldsa65() {
        // Borsh tag / discriminant for ML-DSA-65 must be 2 (matches nearcore).
        assert_eq!(CurveType::MLDSA65 as u8, 2);
        let parsed = CurveType::from_u8(2).unwrap();
        assert_eq!(parsed, CurveType::MLDSA65);
    }

    #[test]
    fn test_parse_public_key_error_invalid_length_message() {
        // Ensure the error message mentions all expected lengths
        let err = ParsePublicKeyError { kind: ParsePublicKeyErrorKind::InvalidLength(48) };
        let msg = err.to_string();
        assert!(msg.contains("48"), "should show actual length");
        assert!(msg.contains("32"), "should mention ed25519 expected length");
        assert!(msg.contains("64"), "should mention secp256k1 expected length");
        assert!(msg.contains("1952"), "should mention ml-dsa-65 expected length");
    }

    #[test]
    fn test_parse_public_key_error_unknown_curve() {
        let err = ParsePublicKeyError { kind: ParsePublicKeyErrorKind::UnknownCurve };
        assert_eq!(err.to_string(), "unknown curve kind");
    }

    #[test]
    fn test_parse_public_key_error_implements_std_error() {
        let err = ParsePublicKeyError { kind: ParsePublicKeyErrorKind::UnknownCurve };
        let _: &dyn std::error::Error = &err;
    }

    #[cfg(feature = "borsh")]
    #[test]
    fn test_public_key_borsh_format_change() {
        // Original struct to reference Borsh serialization from
        #[derive(BorshSerialize, BorshDeserialize)]
        struct PublicKeyRef(Vec<u8>);

        let mut data = vec![CurveType::ED25519 as u8];
        data.extend(
            bs58::decode("6E8sCci9badyRkXb3JoRpBj5p8C6Tw41ELDZoiihKEtp").into_vec().unwrap(),
        );

        // Test internal serialization of Vec<u8> is the same:
        let old_key = PublicKeyRef(data.clone());
        let old_encoded_key = borsh::to_vec(&old_key).unwrap();
        let new_key: PublicKey = data.try_into().unwrap();
        let new_encoded_key = borsh::to_vec(&new_key).unwrap();
        assert_eq!(old_encoded_key, new_encoded_key);
        assert_eq!(
            &new_encoded_key,
            &bs58::decode("279Zpep9MBBg4nKsVmTQE7NbXZkWdxti6HS1yzhp8qnc1ExS7gU")
                .into_vec()
                .unwrap()
        );

        let decoded_key = PublicKey::try_from_slice(&new_encoded_key).unwrap();
        assert_eq!(decoded_key, new_key);
    }

    #[test]
    fn test_public_key_mldsa65_string_roundtrip() {
        // ML-DSA-65 full public keys are 1952 raw bytes.
        let data = vec![1u8; CurveType::MLDSA65.data_len()];
        let key = PublicKey::from_parts(CurveType::MLDSA65, data).unwrap();

        let encoded = key.to_string();
        assert!(encoded.starts_with("ml-dsa-65:"), "expected `ml-dsa-65:` prefix, got {encoded}");

        let parsed = PublicKey::from_str(&encoded).unwrap();
        assert_eq!(parsed, key);
        assert_eq!(parsed.curve_type(), CurveType::MLDSA65);
    }

    #[cfg(feature = "borsh")]
    #[test]
    fn test_public_key_mldsa65_borsh_roundtrip() {
        // Proves `signer_account_pk`-style decoding of a 1953-byte (tag + key)
        // borsh blob works for ML-DSA-65.
        let data = vec![7u8; CurveType::MLDSA65.data_len()];
        let key = PublicKey::from_parts(CurveType::MLDSA65, data).unwrap();

        let encoded = borsh::to_vec(&key).unwrap();
        let decoded = PublicKey::try_from_slice(&encoded).unwrap();
        assert_eq!(decoded, key);
        assert_eq!(decoded.curve_type(), CurveType::MLDSA65);
    }

    #[test]
    fn test_public_key_mldsa65_invalid_length() {
        // A key with the ml-dsa-65 prefix but the wrong number of bytes must error.
        let err = PublicKey::from_parts(CurveType::MLDSA65, vec![1u8; 100]).unwrap_err();
        assert!(matches!(err.kind, ParsePublicKeyErrorKind::InvalidLength(100)));
    }
}
