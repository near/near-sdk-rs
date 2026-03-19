use crate::CryptoHash;
use bs58::decode::Error as B58Error;
use near_sdk_macros::near;
use serde::{Deserialize, de, ser};
use std::convert::TryFrom;

#[near(inside_nearsdk)]
#[derive(Debug, Copy, Clone, PartialEq, PartialOrd, Ord, Eq, Default, Hash)]
#[cfg_attr(feature = "arbitrary", derive(::arbitrary::Arbitrary))]
#[repr(transparent)]
pub struct Base58CryptoHash(CryptoHash);

impl PartialEq<CryptoHash> for Base58CryptoHash {
    fn eq(&self, other: &CryptoHash) -> bool {
        self.0.eq(other)
    }
}

impl PartialOrd<CryptoHash> for Base58CryptoHash {
    fn partial_cmp(&self, other: &CryptoHash) -> Option<std::cmp::Ordering> {
        self.0.partial_cmp(other)
    }
}

impl From<Base58CryptoHash> for CryptoHash {
    fn from(v: Base58CryptoHash) -> Self {
        v.0
    }
}

impl From<Base58CryptoHash> for Vec<u8> {
    fn from(v: Base58CryptoHash) -> Self {
        v.0.into()
    }
}

impl From<&Base58CryptoHash> for CryptoHash {
    fn from(v: &Base58CryptoHash) -> Self {
        v.0
    }
}

impl From<CryptoHash> for Base58CryptoHash {
    fn from(c: CryptoHash) -> Self {
        Base58CryptoHash(c)
    }
}

impl From<&CryptoHash> for Base58CryptoHash {
    fn from(c: &CryptoHash) -> Self {
        Base58CryptoHash(*c)
    }
}

impl AsRef<CryptoHash> for Base58CryptoHash {
    fn as_ref(&self) -> &CryptoHash {
        &self.0
    }
}

impl AsRef<[u8]> for Base58CryptoHash {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl ser::Serialize for Base58CryptoHash {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ser::Serializer,
    {
        serializer.serialize_str(&String::from(self))
    }
}

impl<'de> de::Deserialize<'de> for Base58CryptoHash {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        let s: String = Deserialize::deserialize(deserializer)?;
        s.parse::<Self>().map_err(de::Error::custom)
    }
}

#[cfg(feature = "abi")]
impl schemars::JsonSchema for Base58CryptoHash {
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

impl From<&Base58CryptoHash> for String {
    fn from(hash: &Base58CryptoHash) -> Self {
        bs58::encode(&hash.0).into_string()
    }
}

impl TryFrom<String> for Base58CryptoHash {
    type Error = ParseCryptoHashError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        value.parse()
    }
}

impl TryFrom<&str> for Base58CryptoHash {
    type Error = ParseCryptoHashError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        value.parse()
    }
}

impl std::str::FromStr for Base58CryptoHash {
    type Err = ParseCryptoHashError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let mut crypto_hash: CryptoHash = Default::default();
        let size = bs58::decode(value).onto(&mut crypto_hash)?;
        if size != std::mem::size_of::<CryptoHash>() {
            return Err(ParseCryptoHashError {
                kind: ParseCryptoHashErrorKind::InvalidLength(size),
            });
        }
        Ok(Self(crypto_hash))
    }
}

#[derive(Debug)]
pub struct ParseCryptoHashError {
    kind: ParseCryptoHashErrorKind,
}

#[derive(Debug)]
enum ParseCryptoHashErrorKind {
    InvalidLength(usize),
    Base58(B58Error),
}

impl std::fmt::Display for ParseCryptoHashError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.kind {
            ParseCryptoHashErrorKind::InvalidLength(l) => {
                write!(f, "invalid length of the crypto hash, expected 32 got {l}")
            }
            ParseCryptoHashErrorKind::Base58(e) => write!(f, "base58 decoding error: {e}"),
        }
    }
}

impl From<B58Error> for ParseCryptoHashError {
    fn from(e: B58Error) -> Self {
        Self { kind: ParseCryptoHashErrorKind::Base58(e) }
    }
}

impl std::error::Error for ParseCryptoHashError {}
