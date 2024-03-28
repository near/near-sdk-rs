use borsh::{BorshDeserialize, BorshSerialize};
use bs58::decode::Error as B58Error;
use near_sdk_macros::near;
use std::{convert::TryFrom, io};

/// PublicKey curve
#[near(inside_nearsdk, serializers=[borsh(use_discriminant = true)])]
#[derive(Debug, Clone, Copy, PartialOrd, Ord, Eq, PartialEq)]
#[repr(u8)]
pub enum CurveType {
    ED25519 = 0,
    SECP256K1 = 1,
}

impl CurveType {
    fn from_u8(val: u8) -> Result<Self, ParsePublicKeyError> {
        match val {
            0 => Ok(CurveType::ED25519),
            1 => Ok(CurveType::SECP256K1),
            _ => Err(ParsePublicKeyError { kind: ParsePublicKeyErrorKind::UnknownCurve }),
        }
    }

    /// Get the length of bytes associated to this CurveType
    const fn data_len(&self) -> usize {
        match self {
            CurveType::ED25519 => 32,
            CurveType::SECP256K1 => 64,
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
        } else {
            Err(ParsePublicKeyError { kind: ParsePublicKeyErrorKind::UnknownCurve })
        }
    }
}

#[cfg(all(not(target_arch = "wasm32"), feature = "unit-testing"))]
#[cfg(test)]
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
        }
    }
}

/// Public key in a binary format with base58 string serialization with human-readable curve.
/// The key types currently supported are `secp256k1` and `ed25519`.
///
/// Ed25519 public keys accepted are 32 bytes and secp256k1 keys are the uncompressed 64 format.
///
/// # Example
/// ```
/// use near_sdk::PublicKey;
///
/// // Compressed ed25519 key
/// let ed: PublicKey = "ed25519:6E8sCci9badyRkXb3JoRpBj5p8C6Tw41ELDZoiihKEtp".parse()
///             .unwrap();
///
/// // Uncompressed secp256k1 key
/// let secp256k1: PublicKey = "secp256k1:qMoRgcoXai4mBPsdbHi1wfyxF9TdbPCF4qSDQTRP3TfescSRoUdSx6nmeQoN3aiwGzwMyGXAb1gUjBTv5AY8DXj"
///             .parse()
///             .unwrap();
/// ```
#[derive(Debug, Clone, PartialEq, PartialOrd, Ord, Eq, BorshSerialize, Hash)]
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
        CurveType::from_u8(self.data[0]).unwrap_or_else(|_| crate::env::abort())
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

impl serde::Serialize for PublicKey {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&String::from(self))
    }
}

impl BorshDeserialize for PublicKey {
    fn deserialize_reader<R: io::Read>(reader: &mut R) -> io::Result<Self> {
        <Vec<u8> as BorshDeserialize>::deserialize_reader(reader).and_then(|s| {
            Self::try_from(s).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
        })
    }
}

impl<'de> serde::Deserialize<'de> for PublicKey {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s: String = serde::Deserialize::deserialize(deserializer)?;
        s.parse::<PublicKey>().map_err(serde::de::Error::custom)
    }
}

#[cfg(feature = "abi")]
impl schemars::JsonSchema for PublicKey {
    fn is_referenceable() -> bool {
        false
    }

    fn schema_name() -> String {
        String::schema_name()
    }

    fn json_schema(gen: &mut schemars::gen::SchemaGenerator) -> schemars::schema::Schema {
        String::json_schema(gen)
    }
}

impl From<&PublicKey> for String {
    fn from(str_public_key: &PublicKey) -> Self {
        match str_public_key.curve_type() {
            CurveType::ED25519 => {
                ["ed25519:", &bs58::encode(&str_public_key.data[1..]).into_string()].concat()
            }
            CurveType::SECP256K1 => {
                ["secp256k1:", &bs58::encode(&str_public_key.data[1..]).into_string()].concat()
            }
        }
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
}

impl std::fmt::Display for ParsePublicKeyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.kind {
            ParsePublicKeyErrorKind::InvalidLength(l) => {
                write!(f, "invalid length of the public key, expected 32 got {}", l)
            }
            ParsePublicKeyErrorKind::Base58(e) => write!(f, "base58 decoding error: {}", e),
            ParsePublicKeyErrorKind::UnknownCurve => write!(f, "unknown curve kind"),
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
    use std::convert::TryInto;
    use std::str::FromStr;

    fn expected_key() -> PublicKey {
        let mut key = vec![CurveType::ED25519 as u8];
        key.extend(
            bs58::decode("6E8sCci9badyRkXb3JoRpBj5p8C6Tw41ELDZoiihKEtp").into_vec().unwrap(),
        );
        key.try_into().unwrap()
    }

    #[test]
    fn test_public_key_deser() {
        let key: PublicKey =
            serde_json::from_str("\"ed25519:6E8sCci9badyRkXb3JoRpBj5p8C6Tw41ELDZoiihKEtp\"")
                .unwrap();
        assert_eq!(key, expected_key());
    }

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
}
