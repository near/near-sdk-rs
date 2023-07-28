use borsh::{maybestd::io, BorshDeserialize, BorshSchema, BorshSerialize};
use serde::{de, Deserialize, Serialize};
use std::borrow::{Borrow, Cow};
use std::convert::TryFrom;
use std::fmt;
use std::ops::{Deref, DerefMut};

use crate::env::is_valid_account_id;

/// Account identifier. This is the human readable utf8 string which is used internally to index
/// accounts on the network and their respective state.
///
/// Because these IDs have to be validated, they have to be converted from a string
/// with [`FromStr`] or [`TryFrom`] a compatible type. To skip validation on initialization,
/// [`AccountId::new_unchecked`] can be used.
///
/// This is the "owned" version of the account ID. It is to [`AccountIdRef`] what [`String`] is to [`str`],
/// and works quite similarly to [`std::path::PathBuf`].
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
#[cfg_attr(feature = "abi", derive(schemars::JsonSchema))]
pub struct AccountId(String);

impl AccountId {
    /// Construct an [`AccountId`] from an owned string without validating the address.
    /// It is the responsibility of the caller to ensure the account ID is valid.
    ///
    /// For more information, read: <https://docs.near.org/docs/concepts/account#account-id-rules>
    pub fn new_unchecked(id: String) -> Self {
        debug_assert!(is_valid_account_id(id.as_bytes()));
        Self(id)
    }
}

impl fmt::Display for AccountId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&**self, f)
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

impl AsRef<AccountIdRef> for AccountId {
    fn as_ref(&self) -> &AccountIdRef {
        AccountIdRef::new_unchecked(&self.0)
    }
}

impl AsMut<AccountIdRef> for AccountId {
    fn as_mut(&mut self) -> &mut AccountIdRef {
        AccountIdRef::new_unchecked_mut(&mut self.0)
    }
}

impl Borrow<AccountIdRef> for AccountId {
    fn borrow(&self) -> &AccountIdRef {
        AccountIdRef::new_unchecked(&self.0)
    }
}

impl Deref for AccountId {
    type Target = AccountIdRef;

    fn deref(&self) -> &Self::Target {
        AccountIdRef::new_unchecked(self.0.as_str())
    }
}

impl DerefMut for AccountId {
    fn deref_mut(&mut self) -> &mut Self::Target {
        AccountIdRef::new_unchecked_mut(self.0.as_mut_str())
    }
}

impl<'a> From<AccountId> for Cow<'a, AccountId> {
    fn from(id: AccountId) -> Self {
        Cow::Owned(id)
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

/// Account identifier. This is the human readable utf8 string which is used internally to index
/// accounts on the network and their respective state.
///
/// This is the "referenced" version of the account ID. It is to [`AccountId`] what [`str`] is to [`String`],
/// and works quite similarly to [`std::path::Path`]. Like with [`str`] and [`Path`](std::path::Path), you
/// can't have a value of type `AccountIdRef`, but you can have a reference like `&AccountIdRef` or
/// `&mut AccountIdRef`.
///
/// This type supports zero-copy deserialization offered by [`serde`](https://docs.rs/serde/), but cannot
/// do the same for [`borsh`](https://docs.rs/borsh/) since the latter does not support zero-copy.
///
/// # Examples
/// ```
/// use near_sdk::{AccountId, AccountIdRef};
/// use std::convert::{TryFrom, TryInto};
///
/// // Construction
/// let alice = AccountIdRef::new("alice.near").unwrap();
/// assert!(AccountIdRef::new("invalid.").is_err());
///
/// // Initialize without validating
/// let alice_unchecked = AccountIdRef::new_unchecked("alice.near");
/// assert_eq!(alice, alice_unchecked);
///
/// // Get a reference from an `AccountId`
/// let mut owned = AccountId::new_unchecked("alice.near".to_string());
/// let r#ref: &AccountIdRef = owned.as_ref();
/// let ref_mut: &mut AccountIdRef = owned.as_mut();
/// ```
///
/// [`FromStr`]: std::str::FromStr
#[derive(Debug, PartialEq, PartialOrd, Ord, Eq, BorshSerialize, Serialize, Hash, BorshSchema)]
#[cfg_attr(feature = "abi", derive(schemars::JsonSchema))]
pub struct AccountIdRef(str);

impl AccountIdRef {
    /// Construct a [`&AccountIdRef`](AccountIdRef) from a string reference without validating the address.
    /// It is the responsibility of the caller to ensure the account ID is valid.
    ///
    /// For more information, read: <https://docs.near.org/docs/concepts/account#account-id-rules>
    pub fn new_unchecked<S: AsRef<str> + ?Sized>(id: &S) -> &Self {
        let id = id.as_ref();
        debug_assert!(is_valid_account_id(id.as_bytes()));

        // Safety:
        // - a newtype struct is guaranteed to have the same memory layout as its only
        //   field
        // - the borrow checker will enforce its rules appropriately on the resulting reference
        unsafe { &*(id as *const str as *const Self) }
    }

    /// Construct a [`&AccountIdRef`](AccountIdRef) from a string reference.
    ///
    /// This constructor validates the provided ID, and will produce an error when validation fails.
    pub fn new<S: AsRef<str> + ?Sized>(id: &S) -> Result<&Self, ParseAccountIdError> {
        let id = id.as_ref();
        validate_account_id(id)?;

        // Safety: see `new_unchecked`
        Ok(unsafe { &*(id as *const str as *const Self) })
    }

    /// Construct a [`&mut AccountIdRef`](AccountIdRef) from a mutable string reference without validating
    /// the address. It is the responsibility of the caller to ensure the account ID is valid.
    pub fn new_unchecked_mut<S: AsMut<str> + ?Sized>(id: &mut S) -> &mut Self {
        let id = id.as_mut();
        debug_assert!(is_valid_account_id(id.as_bytes()));

        // Safety: see `new_unchecked`
        unsafe { &mut *(id as *mut str as *mut Self) }
    }

    /// Construct a [`&mut AccountIdRef`](AccountIdRef) from a mutable string reference.
    ///
    /// This constructor validates the provided ID and will produce an error when validation fails.
    pub fn new_mut<S: AsMut<str> + ?Sized>(id: &mut S) -> Result<&mut Self, ParseAccountIdError> {
        let id = id.as_mut();
        validate_account_id(id)?;

        // Safety: see `new_unchecked`
        Ok(unsafe { &mut *(id as *mut str as *mut Self) })
    }

    /// Returns a reference to the account ID bytes.
    pub fn as_bytes(&self) -> &[u8] {
        self.0.as_bytes()
    }

    /// Returns a reference to the account ID string.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for AccountIdRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

impl ToOwned for AccountIdRef {
    type Owned = AccountId;

    fn to_owned(&self) -> Self::Owned {
        AccountId::new_unchecked(self.0.to_string())
    }
}

impl<'a> From<&'a AccountIdRef> for AccountId {
    fn from(id: &'a AccountIdRef) -> Self {
        id.to_owned()
    }
}

impl<'a> From<&'a AccountIdRef> for Cow<'a, AccountIdRef> {
    fn from(id: &'a AccountIdRef) -> Self {
        Cow::Borrowed(id)
    }
}

impl<'s> TryFrom<&'s str> for &'s AccountIdRef {
    type Error = ParseAccountIdError;

    fn try_from(value: &'s str) -> Result<Self, Self::Error> {
        AccountIdRef::new(value)
    }
}

impl<'s> TryFrom<&'s mut str> for &'s mut AccountIdRef {
    type Error = ParseAccountIdError;

    fn try_from(value: &'s mut str) -> Result<Self, Self::Error> {
        AccountIdRef::new_mut(value)
    }
}

impl AsRef<str> for AccountIdRef {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl<'de> Deserialize<'de> for &'de AccountIdRef {
    fn deserialize<D>(deserializer: D) -> Result<Self, <D as de::Deserializer<'de>>::Error>
    where
        D: de::Deserializer<'de>,
    {
        <&str as Deserialize>::deserialize(deserializer)
            .and_then(|s| Self::try_from(s).map_err(de::Error::custom))
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
    fn test_deser_zero_copy() {
        let key: &AccountIdRef = serde_json::from_str("\"alice.near\"").unwrap();
        assert_eq!(key.as_str(), "alice.near");

        let key: Result<&AccountIdRef, _> = serde_json::from_str("Alice.near");
        assert!(key.is_err());
    }

    #[test]
    fn test_ser() {
        let key: AccountId = "alice.near".parse().unwrap();
        let actual: String = serde_json::to_string(&key).unwrap();
        assert_eq!(actual, "\"alice.near\"");

        let key = AccountIdRef::new("alice.near").unwrap();
        let actual: String = serde_json::to_string(key).unwrap();
        assert_eq!(actual, "\"alice.near\"");
    }

    #[test]
    fn test_from_str() {
        let key = "alice.near".parse::<AccountId>().unwrap();
        assert_eq!(key.as_str(), "alice.near");

        let key: &AccountIdRef = "alice.near".try_into().unwrap();
        assert_eq!(key.as_str(), "alice.near");
    }

    #[test]
    fn borsh_serialize_impl() {
        let id = "test.near";
        let account_id = AccountId::new_unchecked(id.to_string());

        // Test to make sure the account ID is serialized as a string through borsh
        assert_eq!(str::try_to_vec(id).unwrap(), account_id.try_to_vec().unwrap());

        let account_id_ref = AccountIdRef::new_unchecked(id);
        assert_eq!(account_id.try_to_vec().unwrap(), account_id_ref.try_to_vec().unwrap());
    }
}
