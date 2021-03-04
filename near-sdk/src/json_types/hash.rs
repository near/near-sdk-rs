use crate::CryptoHash;
use borsh::{BorshDeserialize, BorshSerialize};
use std::convert::{TryFrom, TryInto};

#[derive(
    Debug, Copy, Clone, PartialEq, PartialOrd, Ord, Eq, BorshDeserialize, BorshSerialize, Default,
)]
pub struct Base58CryptoHash(CryptoHash);

impl From<Base58CryptoHash> for CryptoHash {
    fn from(v: Base58CryptoHash) -> CryptoHash {
        v.0
    }
}

impl From<CryptoHash> for Base58CryptoHash {
    fn from(c: CryptoHash) -> Base58CryptoHash {
        Base58CryptoHash(c)
    }
}

impl serde::Serialize for Base58CryptoHash {
    fn serialize<S>(
        &self,
        serializer: S,
    ) -> Result<<S as serde::Serializer>::Ok, <S as serde::Serializer>::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&String::from(self))
    }
}

impl<'de> serde::Deserialize<'de> for Base58CryptoHash {
    fn deserialize<D>(deserializer: D) -> Result<Self, <D as serde::Deserializer<'de>>::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = <String as serde::Deserialize>::deserialize(deserializer)?;
        s.try_into()
            .map_err(|err: Box<dyn std::error::Error>| serde::de::Error::custom(err.to_string()))
    }
}

impl From<&Base58CryptoHash> for String {
    fn from(hash: &Base58CryptoHash) -> Self {
        bs58::encode(&hash.0).into_string()
    }
}

impl TryFrom<String> for Base58CryptoHash {
    type Error = Box<dyn std::error::Error>;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::try_from(value.as_str())
    }
}

impl TryFrom<&str> for Base58CryptoHash {
    type Error = Box<dyn std::error::Error>;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let mut crypto_hash: CryptoHash = CryptoHash::default();
        let size = bs58::decode(value).into(&mut crypto_hash)?;
        if size != std::mem::size_of::<CryptoHash>() {
            return Err("Invalid length of the crypto hash (32)".into());
        }
        Ok(Self(crypto_hash))
    }
}
