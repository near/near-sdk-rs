use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Serialize};
use std::convert::{TryFrom, TryInto};

/// PublicKey curve
#[derive(
    Debug, Clone, Copy, Serialize, Deserialize, PartialEq, BorshDeserialize, BorshSerialize,
)]
pub enum CurveType {
    ED25519 = 0,
    SECP256K1 = 1,
}

impl TryFrom<String> for CurveType {
    type Error = Box<dyn std::error::Error>;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.to_lowercase().as_str() {
            "ed25519" => Ok(CurveType::ED25519),
            "secp256k1" => Ok(CurveType::SECP256K1),
            _ => Err("Unknown curve kind".into()),
        }
    }
}

/// Public key in a binary format with base58 string serialization with human-readable curve.
/// e.g. `ed25519:3tysLvy7KGoE8pznUgXvSHa4vYyGvrDZFcT8jgb8PEQ6`
#[derive(Debug, Clone, PartialEq, PartialOrd, Ord, Eq, BorshDeserialize, BorshSerialize)]
pub struct Base58PublicKey(pub Vec<u8>);

impl Base58PublicKey {
    fn split_key_type_data(value: &str) -> Result<(CurveType, &str), Box<dyn std::error::Error>> {
        if let Some(idx) = value.find(':') {
            let (prefix, key_data) = value.split_at(idx);
            Ok((CurveType::try_from(prefix.to_string())?, &key_data[1..]))
        } else {
            // If there is no Default is ED25519.
            Ok((CurveType::ED25519, value))
        }
    }
}

impl From<Base58PublicKey> for Vec<u8> {
    fn from(v: Base58PublicKey) -> Vec<u8> {
        v.0
    }
}

impl TryFrom<Vec<u8>> for Base58PublicKey {
    type Error = Box<dyn std::error::Error>;

    fn try_from(v: Vec<u8>) -> Result<Self, Self::Error> {
        match v.len() {
            33 if v[0] == 0 => Ok(Self(v)),
            65 if v[0] == 1 => Ok(Self(v)),
            _ => Err("Invalid public key".into()),
        }
    }
}

impl serde::Serialize for Base58PublicKey {
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

impl<'de> serde::Deserialize<'de> for Base58PublicKey {
    fn deserialize<D>(deserializer: D) -> Result<Self, <D as serde::Deserializer<'de>>::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = <String as serde::Deserialize>::deserialize(deserializer)?;
        s.try_into()
            .map_err(|err: Box<dyn std::error::Error>| serde::de::Error::custom(err.to_string()))
    }
}

impl From<&Base58PublicKey> for String {
    fn from(str_public_key: &Base58PublicKey) -> Self {
        match str_public_key.0[0] {
            0 => "ed25519:".to_string() + &bs58::encode(&str_public_key.0[1..]).into_string(),
            1 => "secp256k1:".to_string() + &bs58::encode(&str_public_key.0[1..]).into_string(),
            _ => panic!("Unexpected curve"),
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
        let (key_type, key_data) = Base58PublicKey::split_key_type_data(&value)?;
        let expected_length = match key_type {
            CurveType::ED25519 => 32,
            CurveType::SECP256K1 => 64,
        };
        let data = bs58::decode(key_data).into_vec()?;
        if data.len() != expected_length {
            return Err("Invalid length of the public key".into());
        }
        let mut res = Vec::with_capacity(1 + expected_length);
        match key_type {
            CurveType::ED25519 => res.push(0),
            CurveType::SECP256K1 => res.push(1),
        };
        res.extend(data);
        Ok(Self(res))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn binary_key() -> Vec<u8> {
        let mut binary_key = vec![0];
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
        assert_eq!(key.0, binary_key());
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
        assert_eq!(key.0, binary_key());
    }

    #[test]
    fn test_public_key_to_string() {
        let key: Base58PublicKey = binary_key().try_into().unwrap();
        let actual: String = String::try_from(&key).unwrap();
        assert_eq!(actual, "ed25519:6E8sCci9badyRkXb3JoRpBj5p8C6Tw41ELDZoiihKEtp");
    }
}
