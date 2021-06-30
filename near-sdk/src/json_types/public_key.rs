use borsh::{maybestd::io, BorshDeserialize, BorshSerialize};
use bs58::decode::Error as B58Error;
use std::convert::TryFrom;

/// PublicKey curve
#[derive(Debug, Clone, Copy, PartialOrd, Ord, Eq, PartialEq, BorshDeserialize, BorshSerialize)]
#[repr(u8)]
pub enum CurveType {
    ED25519 = 0,
    SECP256K1 = 1,
}

impl TryFrom<u8> for CurveType {
    type Error = ParsePublicKeyError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::ED25519),
            1 => Ok(Self::SECP256K1),
            _ => Err(ParsePublicKeyError { kind: ParsePublicKeyErrorKind::UnknownCurve }),
        }
    }
}

impl TryFrom<String> for CurveType {
    type Error = ParsePublicKeyError;

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
#[derive(Debug, Clone, PartialEq, PartialOrd, Ord, Eq)]
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

    fn from_parts(curve: CurveType, data: Vec<u8>) -> Result<Self, ParsePublicKeyError> {
        let expected_length = match curve {
            CurveType::ED25519 => 32,
            CurveType::SECP256K1 => 64,
        };
        if data.len() != expected_length {
            return Err(ParsePublicKeyError {
                kind: ParsePublicKeyErrorKind::InvalidLength(data.len()),
            });
        }

        Ok(Self { curve, data })
    }

    /// Get info about the CurveType for this public key
    pub fn curve(&self) -> CurveType {
        self.curve
    }
}

impl From<Base58PublicKey> for Vec<u8> {
    fn from(mut v: Base58PublicKey) -> Vec<u8> {
        v.data.insert(0, v.curve as u8);
        v.data
    }
}

impl TryFrom<Vec<u8>> for Base58PublicKey {
    type Error = ParsePublicKeyError;

    fn try_from(mut data: Vec<u8>) -> Result<Self, Self::Error> {
        if data.len() == 0 {
            return Err(ParsePublicKeyError {
                kind: ParsePublicKeyErrorKind::InvalidLength(data.len()),
            });
        }
        let curve = CurveType::try_from(data.remove(0))?;
        Self::from_parts(curve, data)
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
        let s: String = serde::Deserialize::deserialize(deserializer)?;
        s.parse::<Base58PublicKey>().map_err(serde::de::Error::custom)
    }
}

impl From<&Base58PublicKey> for String {
    fn from(str_public_key: &Base58PublicKey) -> Self {
        match str_public_key.curve {
            CurveType::ED25519 => {
                ["ed25519:", &bs58::encode(&str_public_key.data).into_string()].concat()
            }
            CurveType::SECP256K1 => {
                ["secp256k1:", &bs58::encode(&str_public_key.data).into_string()].concat()
            }
        }
    }
}

impl TryFrom<String> for Base58PublicKey {
    type Error = ParsePublicKeyError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::try_from(value.as_str())
    }
}

impl TryFrom<&str> for Base58PublicKey {
    type Error = ParsePublicKeyError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Ok(value.parse::<Self>()?)
    }
}

impl std::str::FromStr for Base58PublicKey {
    type Err = ParsePublicKeyError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let (curve, key_data) = Base58PublicKey::split_key_type_data(&value)?;
        let data = bs58::decode(key_data).into_vec()?;
        Self::from_parts(curve, data)
    }
}

impl BorshDeserialize for Base58PublicKey {
    fn deserialize(buf: &mut &[u8]) -> io::Result<Self> {
        Ok(Self { curve: CurveType::deserialize(buf)?, data: BorshDeserialize::deserialize(buf)? })
    }
}

impl BorshSerialize for Base58PublicKey {
    fn serialize<W: io::Write>(&self, writer: &mut W) -> io::Result<()> {
        self.curve.serialize(writer)?;
        BorshSerialize::serialize(&self.data, writer)
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

    fn expected_key() -> Base58PublicKey {
        let mut key = vec![CurveType::ED25519 as u8];
        key.extend(
            bs58::decode("6E8sCci9badyRkXb3JoRpBj5p8C6Tw41ELDZoiihKEtp").into_vec().unwrap(),
        );
        key.try_into().unwrap()
    }

    #[test]
    fn test_public_key_deser() {
        let key: Base58PublicKey =
            serde_json::from_str("\"ed25519:6E8sCci9badyRkXb3JoRpBj5p8C6Tw41ELDZoiihKEtp\"")
                .unwrap();
        assert_eq!(key, expected_key());
    }

    #[test]
    fn test_public_key_ser() {
        let key: Base58PublicKey = expected_key();
        let actual: String = serde_json::to_string(&key).unwrap();
        assert_eq!(actual, "\"ed25519:6E8sCci9badyRkXb3JoRpBj5p8C6Tw41ELDZoiihKEtp\"");
    }

    #[test]
    fn test_public_key_from_str() {
        let key = Base58PublicKey::try_from("ed25519:6E8sCci9badyRkXb3JoRpBj5p8C6Tw41ELDZoiihKEtp")
            .unwrap();
        assert_eq!(key, expected_key());
    }

    #[test]
    fn test_public_key_to_string() {
        let key: Base58PublicKey = expected_key();
        let actual: String = String::try_from(&key).unwrap();
        assert_eq!(actual, "ed25519:6E8sCci9badyRkXb3JoRpBj5p8C6Tw41ELDZoiihKEtp");
    }

    #[test]
    fn test_public_key_borsh() {
        let key: Base58PublicKey = expected_key();
        let encoded_key = key.clone().try_to_vec().unwrap();
        let decoded_key = Base58PublicKey::try_from_slice(&encoded_key).unwrap();
        assert_eq!(key, decoded_key);
    }
}
