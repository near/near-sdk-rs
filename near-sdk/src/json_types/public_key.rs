use borsh::{BorshDeserialize, BorshSerialize};
use bs58::decode::Error as B58Error;
use serde::{Deserialize, Serialize};
use std::convert::TryFrom;

/// PublicKey curve
#[derive(
    Debug,
    Clone,
    Copy,
    Serialize,
    Deserialize,
    PartialOrd,
    Ord,
    Eq,
    PartialEq,
    BorshDeserialize,
    BorshSerialize,
)]
pub enum CurveType {
    ED25519 = 0,
    SECP256K1 = 1,
}

impl TryFrom<String> for CurveType {
    type Error = Box<dyn std::error::Error>;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Ok(value.parse::<Self>()?)
    }
}

impl std::str::FromStr for CurveType {
    type Err = ParsePublicKeyError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.to_lowercase().as_str() {
            "ed25519" => Ok(CurveType::ED25519),
            "secp256k1" => Ok(CurveType::SECP256K1),
            _ => Err(ParsePublicKeyError { kind: ParsePublicKeyErrorKind::UnknownCurve }),
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
/// use near_sdk::json_types::Base58PublicKey;
///
/// // Compressed ed25519 key
/// let ed: Base58PublicKey = "ed25519:6E8sCci9badyRkXb3JoRpBj5p8C6Tw41ELDZoiihKEtp".parse()
///             .unwrap();
///
/// // Uncompressed secp256k1 key
/// let secp256k1: Base58PublicKey  = "secp256k1:qMoRgcoXai4mBPsdbHi1wfyxF9TdbPCF4qSDQTRP3TfescSRoUdSx6nmeQoN3aiwGzwMyGXAb1gUjBTv5AY8DXj"
///             .parse()
///             .unwrap();
/// ```
#[derive(Debug, Clone, PartialEq, PartialOrd, Ord, Eq, BorshDeserialize, BorshSerialize)]
pub struct Base58PublicKey {
    curve: CurveType,
    data: Vec<u8>,
}

impl Base58PublicKey {
    fn split_key_type_data(value: &str) -> Result<(CurveType, &str), ParsePublicKeyError> {
        if let Some(idx) = value.find(':') {
            let (prefix, key_data) = value.split_at(idx);
            Ok((prefix.parse::<CurveType>()?, &key_data[1..]))
        } else {
            // If there is no Default is ED25519.
            Ok((CurveType::ED25519, value))
        }
    }
}

impl From<Base58PublicKey> for Vec<u8> {
    fn from(v: Base58PublicKey) -> Vec<u8> {
        let mut out = vec![v.curve as u8];
        out.extend(v.data);
        out
    }
}

impl TryFrom<Vec<u8>> for Base58PublicKey {
    type Error = Box<dyn std::error::Error>;

    fn try_from(v: Vec<u8>) -> Result<Self, Self::Error> {
        match &*v {
            [0, other @ ..] => Ok(Self { curve: CurveType::ED25519, data: other.into() }),
            [1, other @ ..] => Ok(Self { curve: CurveType::SECP256K1, data: other.into() }),
            _ => Err("Invalid public key".into()),
        }
    }
}

impl serde::Serialize for Base58PublicKey {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&String::from(self))
    }
}

impl<'de> serde::Deserialize<'de> for Base58PublicKey {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s: String = Deserialize::deserialize(deserializer)?;
        s.parse::<Base58PublicKey>().map_err(serde::de::Error::custom)
    }
}

impl From<&Base58PublicKey> for String {
    fn from(str_public_key: &Base58PublicKey) -> Self {
        match str_public_key.curve {
            CurveType::ED25519 => {
                "ed25519:".to_string() + &bs58::encode(&str_public_key.data).into_string()
            }
            CurveType::SECP256K1 => {
                "secp256k1:".to_string() + &bs58::encode(&str_public_key.data).into_string()
            }
        }
    }
}

impl TryFrom<String> for Base58PublicKey {
    type Error = Box<dyn std::error::Error>;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::try_from(value.as_str())
    }
}

impl TryFrom<&str> for Base58PublicKey {
    type Error = Box<dyn std::error::Error>;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Ok(value.parse::<Self>()?)
    }
}

impl std::str::FromStr for Base58PublicKey {
    type Err = ParsePublicKeyError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let (key_type, key_data) = Base58PublicKey::split_key_type_data(&value)?;
        let expected_length = match key_type {
            CurveType::ED25519 => 32,
            CurveType::SECP256K1 => 64,
        };
        let data = bs58::decode(key_data).into_vec()?;
        if data.len() != expected_length {
            return Err(ParsePublicKeyError {
                kind: ParsePublicKeyErrorKind::InvalidLength(data.len()),
            });
        }
        Ok(Self { curve: key_type, data })
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

    fn binary_key() -> Vec<u8> {
        let mut binary_key = vec![CurveType::ED25519 as u8];
        binary_key.extend(
            bs58::decode("6E8sCci9badyRkXb3JoRpBj5p8C6Tw41ELDZoiihKEtp").into_vec().unwrap(),
        );
        binary_key
    }

    #[test]
    fn test_public_key_deser() {
        let key: Base58PublicKey =
            serde_json::from_str("\"ed25519:6E8sCci9badyRkXb3JoRpBj5p8C6Tw41ELDZoiihKEtp\"")
                .unwrap();
        assert_eq!(Vec::<u8>::from(key), binary_key());
    }

    #[test]
    fn test_public_key_ser() {
        let key: Base58PublicKey = binary_key().try_into().unwrap();
        let actual: String = serde_json::to_string(&key).unwrap();
        assert_eq!(actual, "\"ed25519:6E8sCci9badyRkXb3JoRpBj5p8C6Tw41ELDZoiihKEtp\"");
    }

    #[test]
    fn test_public_key_from_str() {
        let key = Base58PublicKey::try_from("ed25519:6E8sCci9badyRkXb3JoRpBj5p8C6Tw41ELDZoiihKEtp")
            .unwrap();
        assert_eq!(Vec::<u8>::from(key), binary_key());
    }

    #[test]
    fn test_public_key_to_string() {
        let key: Base58PublicKey = binary_key().try_into().unwrap();
        let actual: String = String::try_from(&key).unwrap();
        assert_eq!(actual, "ed25519:6E8sCci9badyRkXb3JoRpBj5p8C6Tw41ELDZoiihKEtp");
    }
}
