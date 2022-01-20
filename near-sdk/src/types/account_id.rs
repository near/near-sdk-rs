use borsh::{maybestd::io, BorshDeserialize, BorshSchema, BorshSerialize};
use serde::{de, Deserialize, Serialize};
use std::convert::TryFrom;
use std::fmt;

use crate::env::is_valid_account_id;

/// Account identifier. This is the human readable utf8 string which is used internally to index
/// accounts on the network and their respective state.
///
/// Because these IDs have to be validated, they have to be converted from a string
/// with [`FromStr`] or [`TryFrom`] a compatible type. To skip validation on initialization,
/// [`AccountId::new_unchecked`] can be used.
///
/// # Examples
/// ```
/// use near_sdk::AccountId;
/// use std::convert::{TryFrom, TryInto};
///
/// // `FromStr` conversion
/// let alice: AccountId = "alice.near".parse().unwrap();
/// assert!("invalid.".parse::<AccountId>().is_err());
///
/// let alice_string = "alice".to_string();
///
/// // From string with validation
/// let alice = AccountId::try_from(alice_string.clone()).unwrap();
/// let alice: AccountId = alice_string.try_into().unwrap();
///
/// // Initialize without validating
/// let alice_unchecked = AccountId::new_unchecked("alice".to_string());
/// assert_eq!(alice, alice_unchecked);
/// ```
///
/// [`FromStr`]: std::str::FromStr
#[derive(
    Debug, Clone, PartialEq, PartialOrd, Ord, Eq, BorshSerialize, Serialize, Hash, BorshSchema,
)]
pub struct AccountId(String);

impl AccountId {
    /// Returns reference to the account ID bytes.
    pub fn as_bytes(&self) -> &[u8] {
        self.0.as_bytes()
    }
    /// Returns reference to the account ID string.
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
    /// Caller must ensure that the account id is valid.
    ///
    /// For more information, read: <https://docs.near.org/docs/concepts/account#account-id-rules>
    pub fn new_unchecked(id: String) -> Self {
        debug_assert!(is_valid_account_id(id.as_bytes()));
        Self(id)
    }
}

impl fmt::Display for AccountId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

impl From<AccountId> for String {
    fn from(id: AccountId) -> Self {
        id.0
    }
}

impl AsRef<str> for AccountId {
    fn as_ref(&self) -> &str {
        self.0.as_str()
    }
}

impl<'de> Deserialize<'de> for AccountId {
    fn deserialize<D>(deserializer: D) -> Result<Self, <D as de::Deserializer<'de>>::Error>
    where
        D: de::Deserializer<'de>,
    {
        <String as Deserialize>::deserialize(deserializer)
            .and_then(|s| Self::try_from(s).map_err(de::Error::custom))
    }
}

impl BorshDeserialize for AccountId {
    fn deserialize(buf: &mut &[u8]) -> io::Result<Self> {
        <String as BorshDeserialize>::deserialize(buf).and_then(|s| {
            Self::try_from(s).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
        })
    }
}

fn validate_account_id(id: &str) -> Result<(), ParseAccountIdError> {
    if is_valid_account_id(id.as_bytes()) {
        Ok(())
    } else {
        Err(ParseAccountIdError {})
    }
}

impl TryFrom<String> for AccountId {
    type Error = ParseAccountIdError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        validate_account_id(value.as_str())?;
        Ok(Self(value))
    }
}

impl std::str::FromStr for AccountId {
    type Err = ParseAccountIdError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        validate_account_id(value)?;
        Ok(Self(value.to_string()))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct ParseAccountIdError {}

impl fmt::Display for ParseAccountIdError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "the account ID is invalid")
    }
}

impl std::error::Error for ParseAccountIdError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deser() {
        let key: AccountId = serde_json::from_str("\"alice.near\"").unwrap();
        assert_eq!(key.0, "alice.near".to_string());

        let key: Result<AccountId, _> = serde_json::from_str("Alice.near");
        assert!(key.is_err());
    }

    #[test]
    fn test_ser() {
        let key: AccountId = "alice.near".parse().unwrap();
        let actual: String = serde_json::to_string(&key).unwrap();
        assert_eq!(actual, "\"alice.near\"");
    }

    #[test]
    fn test_from_str() {
        let key = "alice.near".parse::<AccountId>().unwrap();
        assert_eq!(key.as_ref(), &"alice.near".to_string());
    }

    #[test]
    fn borsh_serialize_impl() {
        let id = "test.near";
        let account_id = AccountId::new_unchecked(id.to_string());

        // Test to make sure the account ID is serialized as a string through borsh
        assert_eq!(str::try_to_vec(id).unwrap(), account_id.try_to_vec().unwrap());
    }
}
