use borsh::{BorshDeserialize, BorshSerialize};
use serde::Serialize;
use std::convert::{TryFrom, TryInto};

use crate::env::is_valid_account_id;
use crate::AccountId;

/// Helper class to validate account ID during serialization and deserializiation
#[derive(
    Debug, Clone, PartialEq, PartialOrd, Ord, Eq, BorshDeserialize, BorshSerialize, Serialize,
)]
pub struct ValidAccountId(AccountId);

impl ValidAccountId {
    fn is_valid(&self) -> bool {
        is_valid_account_id(&self.0.as_bytes())
    }
}

impl AsRef<AccountId> for ValidAccountId {
    fn as_ref(&self) -> &AccountId {
        &self.0
    }
}

impl<'de> serde::Deserialize<'de> for ValidAccountId {
    fn deserialize<D>(deserializer: D) -> Result<Self, <D as serde::Deserializer<'de>>::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = <String as serde::Deserialize>::deserialize(deserializer)?;
        s.try_into()
            .map_err(|err: Box<dyn std::error::Error>| serde::de::Error::custom(err.to_string()))
    }
}

impl TryFrom<&str> for ValidAccountId {
    type Error = Box<dyn std::error::Error>;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::try_from(value.to_string())
    }
}

impl TryFrom<String> for ValidAccountId {
    type Error = Box<dyn std::error::Error>;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        let res = Self(value);
        if res.is_valid() {
            Ok(res)
        } else {
            Err("The account ID is invalid".into())
        }
    }
}

impl From<ValidAccountId> for AccountId {
    fn from(value: ValidAccountId) -> Self {
        value.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::convert::TryInto;

    #[test]
    fn test_deser() {
        let key: ValidAccountId = serde_json::from_str("\"alice.near\"").unwrap();
        assert_eq!(key.0, "alice.near".to_string());

        let key: Result<ValidAccountId, _> = serde_json::from_str("Alice.near");
        assert!(key.is_err());
    }

    #[test]
    fn test_ser() {
        let key: ValidAccountId = "alice.near".try_into().unwrap();
        let actual: String = serde_json::to_string(&key).unwrap();
        assert_eq!(actual, "\"alice.near\"");
    }

    #[test]
    fn test_from_str() {
        let key = ValidAccountId::try_from("alice.near").unwrap();
        assert_eq!(key.as_ref(), &"alice.near".to_string());
    }
}
