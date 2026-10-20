//! Length-bounded wrappers around [`String`] and [`Vec`].
//!
//! Contract inputs are untrusted. A plain `String` or `Vec<T>` field accepts arbitrarily large
//! values, which inflate storage and gas costs or grow hashed data without bound. [`BoundedString`]
//! and [`BoundedVec`] enforce a compile-time maximum on construction and on every deserialization
//! path (JSON through `serde`, state and arguments through `borsh`), so an oversized value is
//! rejected before it reaches contract logic.

use std::fmt;
use std::ops::Deref;
use std::str::FromStr;

/// Error returned when a value does not fit into a [`BoundedString`] or [`BoundedVec`].
///
/// `len` is the length of the rejected value and `max` the bound it violated. For
/// [`BoundedString`] both are measured in UTF-8 bytes; for [`BoundedVec`] in elements.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TooLongError {
    /// Length of the rejected value.
    pub len: usize,
    /// Maximum length allowed by the bound.
    pub max: usize,
}

impl fmt::Display for TooLongError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ERR_TOO_LONG: length {} exceeds the maximum of {}", self.len, self.max)
    }
}

impl std::error::Error for TooLongError {}

#[cfg(feature = "borsh")]
impl From<TooLongError> for std::io::Error {
    fn from(e: TooLongError) -> Self {
        std::io::Error::new(std::io::ErrorKind::InvalidData, e)
    }
}

/// Helper that compares provided string length and max length. Returns `TooLongError` if len > max.
#[inline]
#[allow(unused)]
fn check_len(len: usize, max: usize) -> Result<(), TooLongError> {
    if len > max { Err(TooLongError { len, max }) } else { Ok(()) }
}

/// UTF-8 string whose byte length is bounded by `MAX`.
///
/// The bound is enforced on construction ([`BoundedString::new`], [`FromStr`], [`TryFrom`]) and
/// on both JSON and Borsh deserialization. Oversized inputs fail with [`TooLongError`].
///
/// Note that `MAX` counts UTF-8 **bytes**, not characters, since bytes are what storage and gas
/// are charged for.
///
/// # Example
/// ```rust
/// use near_sdk_core::types::BoundedString;
///
/// type Memo = BoundedString<16>;
///
/// let memo = Memo::new("short").unwrap();
/// assert_eq!(memo.as_str(), "short");
///
/// assert!(Memo::new("this memo is longer than sixteen bytes").is_err());
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize), serde(transparent))]
pub struct BoundedString<const MAX: usize>(String);

impl<const MAX: usize> BoundedString<MAX> {
    /// Maximum byte length accepted by this type.
    pub const MAX_LEN: usize = MAX;

    /// Wraps `s`, failing with [`TooLongError`] if its UTF-8 byte length exceeds `MAX`.
    pub fn new(s: impl Into<String>) -> Result<Self, TooLongError> {
        let s = s.into();
        if s.len() > MAX {
            return Err(TooLongError { len: s.len(), max: MAX });
        }
        Ok(Self(s))
    }

    /// Returns the wrapped string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Unwraps into the inner [`String`].
    pub fn into_inner(self) -> String {
        self.0
    }
}

impl<const MAX: usize> fmt::Display for BoundedString<MAX> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl<const MAX: usize> Deref for BoundedString<MAX> {
    type Target = str;

    fn deref(&self) -> &str {
        &self.0
    }
}

impl<const MAX: usize> AsRef<str> for BoundedString<MAX> {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl<const MAX: usize> AsRef<[u8]> for BoundedString<MAX> {
    fn as_ref(&self) -> &[u8] {
        self.0.as_bytes()
    }
}

impl<const MAX: usize> From<BoundedString<MAX>> for String {
    fn from(value: BoundedString<MAX>) -> Self {
        value.0
    }
}

impl<const MAX: usize> TryFrom<String> for BoundedString<MAX> {
    type Error = TooLongError;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        Self::new(s)
    }
}

impl<const MAX: usize> TryFrom<&str> for BoundedString<MAX> {
    type Error = TooLongError;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        Self::new(s)
    }
}

impl<const MAX: usize> FromStr for BoundedString<MAX> {
    type Err = TooLongError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::new(s)
    }
}

impl<const MAX: usize> PartialEq<str> for BoundedString<MAX> {
    fn eq(&self, other: &str) -> bool {
        self.0 == other
    }
}

impl<const MAX: usize> PartialEq<&str> for BoundedString<MAX> {
    fn eq(&self, other: &&str) -> bool {
        self.0 == *other
    }
}

/// The bound is checked on the borrowed `&str` the deserializer hands over, before it is copied
/// into an owned `String`. With `serde_json::from_slice` (what `#[near]` uses for arguments) an
/// unescaped string is borrowed straight from the input, so an oversized one is rejected without
/// any allocation.
#[cfg(feature = "serde")]
impl<'de, const MAX: usize> serde::Deserialize<'de> for BoundedString<MAX> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct Visitor<const MAX: usize>;

        impl<'de, const MAX: usize> serde::de::Visitor<'de> for Visitor<MAX> {
            type Value = BoundedString<MAX>;

            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "a string of at most {MAX} bytes")
            }

            fn visit_str<E: serde::de::Error>(self, s: &str) -> Result<Self::Value, E> {
                check_len(s.len(), MAX).map_err(E::custom)?;
                Ok(BoundedString(s.to_owned()))
            }

            fn visit_string<E: serde::de::Error>(self, s: String) -> Result<Self::Value, E> {
                BoundedString::new(s).map_err(E::custom)
            }

            fn visit_bytes<E: serde::de::Error>(self, v: &[u8]) -> Result<Self::Value, E> {
                check_len(v.len(), MAX).map_err(E::custom)?;
                match std::str::from_utf8(v) {
                    Ok(s) => Ok(BoundedString(s.to_owned())),
                    Err(_) => Err(E::invalid_value(serde::de::Unexpected::Bytes(v), &self)),
                }
            }

            fn visit_byte_buf<E: serde::de::Error>(self, v: Vec<u8>) -> Result<Self::Value, E> {
                check_len(v.len(), MAX).map_err(E::custom)?;
                match String::from_utf8(v) {
                    Ok(s) => Ok(BoundedString(s)),
                    Err(e) => {
                        Err(E::invalid_value(serde::de::Unexpected::Bytes(&e.into_bytes()), &self))
                    }
                }
            }
        }

        deserializer.deserialize_string(Visitor::<MAX>)
    }
}

#[cfg(feature = "borsh")]
impl<const MAX: usize> borsh::BorshSerialize for BoundedString<MAX> {
    fn serialize<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()> {
        self.0.serialize(writer)
    }
}

#[cfg(feature = "borsh")]
impl<const MAX: usize> borsh::BorshDeserialize for BoundedString<MAX> {
    fn deserialize_reader<R: std::io::Read>(reader: &mut R) -> std::io::Result<Self> {
        // Before allocating and parsing anything, we will read length of the string first,
        // which is u32 according to borsh spec.
        let len = u32::deserialize_reader(reader)?;
        // Shouldn't be possible as `u32 == usize` on wasm, but we still check.
        let len = usize::try_from(len).map_err(|_| {
            std::io::Error::new(borsh::io::ErrorKind::InvalidData, "length overflows usize")
        })?;

        check_len(len, MAX)?;

        let mut buf = vec![0u8; len];
        reader.read_exact(&mut buf)?;

        String::from_utf8(buf)
            .map(Self)
            .map_err(|e| std::io::Error::new(borsh::io::ErrorKind::InvalidData, e.to_string()))
    }
}

/// Borsh wire format is identical to `String`; only the accepted length differs.
#[cfg(feature = "abi")]
impl<const MAX: usize> borsh::BorshSchema for BoundedString<MAX> {
    fn add_definitions_recursively(
        definitions: &mut std::collections::BTreeMap<
            borsh::schema::Declaration,
            borsh::schema::Definition,
        >,
    ) {
        String::add_definitions_recursively(definitions);
    }

    fn declaration() -> borsh::schema::Declaration {
        String::declaration()
    }
}

/// JSON schema is a `string` with `maxLength` set to `MAX`.
///
/// JSON Schema counts characters while `MAX` counts bytes, so the advertised limit is an upper
/// bound: every accepted value satisfies it, but a value that satisfies it may still be rejected.
#[cfg(feature = "schemars-v0_8")]
impl<const MAX: usize> schemars_v0_8::JsonSchema for BoundedString<MAX> {
    fn is_referenceable() -> bool {
        false
    }

    fn schema_name() -> String {
        format!("BoundedString<{MAX}>")
    }

    fn json_schema(
        r#gen: &mut schemars_v0_8::r#gen::SchemaGenerator,
    ) -> schemars_v0_8::schema::Schema {
        let mut schema = String::json_schema(r#gen);
        if let schemars_v0_8::schema::Schema::Object(obj) = &mut schema {
            obj.string().max_length = u32::try_from(MAX).ok();
        }
        schema
    }
}

#[cfg(feature = "arbitrary")]
impl<'a, const MAX: usize> arbitrary::Arbitrary<'a> for BoundedString<MAX> {
    fn arbitrary(u: &mut arbitrary::Unstructured<'a>) -> arbitrary::Result<Self> {
        let mut s = String::arbitrary(u)?;
        if s.len() > MAX {
            let mut end = MAX;
            while !s.is_char_boundary(end) {
                end -= 1;
            }
            s.truncate(end);
        }
        Ok(Self(s))
    }
}

/// Vector whose element count is bounded by `MAX`.
///
/// The bound is enforced on construction ([`BoundedVec::new`], [`TryFrom`]) and on both JSON and
/// Borsh deserialization. Oversized inputs fail with [`TooLongError`].
///
/// Only the number of elements is bounded. If the elements themselves are variable-size, bound
/// them too (for example `BoundedVec<BoundedString<64>, 8>`).
///
/// # Example
/// ```rust
/// use near_sdk_core::types::BoundedVec;
///
/// type Ids = BoundedVec<u64, 4>;
///
/// let ids = Ids::new(vec![1, 2, 3]).unwrap();
/// assert_eq!(ids.as_slice(), &[1, 2, 3]);
///
/// assert!(Ids::new(vec![1, 2, 3, 4, 5]).is_err());
/// ```
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize), serde(transparent))]
pub struct BoundedVec<T, const MAX: usize>(Vec<T>);

impl<T, const MAX: usize> BoundedVec<T, MAX> {
    /// Maximum number of elements accepted by this type.
    pub const MAX_LEN: usize = MAX;

    /// Wraps `v`, failing with [`TooLongError`] if it holds more than `MAX` elements.
    pub fn new(v: impl Into<Vec<T>>) -> Result<Self, TooLongError> {
        let v = v.into();
        if v.len() > MAX {
            return Err(TooLongError { len: v.len(), max: MAX });
        }
        Ok(Self(v))
    }

    /// Returns the wrapped elements as a slice.
    pub fn as_slice(&self) -> &[T] {
        &self.0
    }

    /// Unwraps into the inner [`Vec`].
    pub fn into_inner(self) -> Vec<T> {
        self.0
    }

    /// Appends `value`, failing with [`TooLongError`] if the vector is already at `MAX`.
    ///
    /// On failure the vector is unchanged and `value` is returned inside the error tuple so the
    /// caller can recover it.
    pub fn try_push(&mut self, value: T) -> Result<(), (TooLongError, T)> {
        if self.0.len() >= MAX {
            return Err((TooLongError { len: self.0.len() + 1, max: MAX }, value));
        }
        self.0.push(value);
        Ok(())
    }

    /// Removes the last element and returns it, or `None` if the vector is empty.
    pub fn pop(&mut self) -> Option<T> {
        self.0.pop()
    }

    /// Removes all elements.
    pub fn clear(&mut self) {
        self.0.clear();
    }
}

// Manual impl so `T: Default` is not required.
impl<T, const MAX: usize> Default for BoundedVec<T, MAX> {
    fn default() -> Self {
        Self(Vec::new())
    }
}

impl<T, const MAX: usize> Deref for BoundedVec<T, MAX> {
    type Target = [T];

    fn deref(&self) -> &[T] {
        &self.0
    }
}

impl<T, const MAX: usize> AsRef<[T]> for BoundedVec<T, MAX> {
    fn as_ref(&self) -> &[T] {
        &self.0
    }
}

impl<T, const MAX: usize> From<BoundedVec<T, MAX>> for Vec<T> {
    fn from(value: BoundedVec<T, MAX>) -> Self {
        value.0
    }
}

impl<T, const MAX: usize> TryFrom<Vec<T>> for BoundedVec<T, MAX> {
    type Error = TooLongError;

    fn try_from(v: Vec<T>) -> Result<Self, Self::Error> {
        Self::new(v)
    }
}

impl<T, const MAX: usize> IntoIterator for BoundedVec<T, MAX> {
    type Item = T;
    type IntoIter = std::vec::IntoIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<'a, T, const MAX: usize> IntoIterator for &'a BoundedVec<T, MAX> {
    type Item = &'a T;
    type IntoIter = std::slice::Iter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

impl<T: PartialEq, const MAX: usize> PartialEq<[T]> for BoundedVec<T, MAX> {
    fn eq(&self, other: &[T]) -> bool {
        self.0 == other
    }
}

impl<T: PartialEq, const MAX: usize> PartialEq<Vec<T>> for BoundedVec<T, MAX> {
    fn eq(&self, other: &Vec<T>) -> bool {
        &self.0 == other
    }
}

#[cfg(feature = "serde")]
impl<'de, T, const MAX: usize> serde::Deserialize<'de> for BoundedVec<T, MAX>
where
    T: serde::Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::Error;

        struct Visitor<T, const MAX: usize>(std::marker::PhantomData<T>);

        impl<'de, T, const MAX: usize> serde::de::Visitor<'de> for Visitor<T, MAX>
        where
            T: serde::Deserialize<'de>,
        {
            type Value = BoundedVec<T, MAX>;

            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "a sequence of at most {MAX} elements")
            }

            fn visit_seq<A: serde::de::SeqAccess<'de>>(
                self,
                mut seq: A,
            ) -> Result<Self::Value, A::Error> {
                let hint = seq.size_hint().unwrap_or(0);
                check_len(hint, MAX).map_err(A::Error::custom)?;

                let mut v = Vec::with_capacity(hint);

                loop {
                    if v.len() == MAX {
                        // just do a peak into the next element.
                        if seq.next_element::<serde::de::IgnoredAny>()?.is_some() {
                            return Err(A::Error::custom(TooLongError { len: MAX + 1, max: MAX }));
                        }
                        break;
                    }

                    match seq.next_element()? {
                        Some(item) => v.push(item),
                        None => break,
                    }
                }

                Ok(BoundedVec(v))
            }
        }

        deserializer.deserialize_seq(Visitor::<T, MAX>(std::marker::PhantomData))
    }
}

#[cfg(feature = "borsh")]
impl<T: borsh::BorshSerialize, const MAX: usize> borsh::BorshSerialize for BoundedVec<T, MAX> {
    fn serialize<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()> {
        self.0.serialize(writer)
    }
}

#[cfg(feature = "borsh")]
impl<T: borsh::BorshDeserialize, const MAX: usize> borsh::BorshDeserialize for BoundedVec<T, MAX> {
    fn deserialize_reader<R: std::io::Read>(reader: &mut R) -> std::io::Result<Self> {
        // We check if the type `T` has size of 0 and reject it in such case - this is
        // to ensure that DoS attacks are not possible. This is the same what is done
        // in borsh.
        if std::mem::size_of::<T>() == 0 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Collections of zero-sized types are not allowed due to deny-of-service concerns on deseri
alization.",
            ));
        }

        // Before allocating and parsing anything, we will read length of the string first,
        // which is u32 according to borsh spec.
        let len = u32::deserialize_reader(reader)?;
        // Shouldn't be possible as `u32 == usize` on wasm, but we still check.
        let len = usize::try_from(len).map_err(|_| {
            std::io::Error::new(borsh::io::ErrorKind::InvalidData, "length overflows usize")
        })?;

        check_len(len, MAX)?;

        let mut v = Vec::with_capacity(len);
        for _ in 0..len {
            v.push(T::deserialize_reader(reader)?)
        }
        Ok(Self(v))
    }
}

/// Borsh wire format is identical to `Vec<T>`; only the accepted length differs.
#[cfg(feature = "abi")]
impl<T: borsh::BorshSchema, const MAX: usize> borsh::BorshSchema for BoundedVec<T, MAX> {
    fn add_definitions_recursively(
        definitions: &mut std::collections::BTreeMap<
            borsh::schema::Declaration,
            borsh::schema::Definition,
        >,
    ) {
        Vec::<T>::add_definitions_recursively(definitions);
    }

    fn declaration() -> borsh::schema::Declaration {
        Vec::<T>::declaration()
    }
}

/// JSON schema is an `array` of `T` with `maxItems` set to `MAX`.
#[cfg(feature = "schemars-v0_8")]
impl<T: schemars_v0_8::JsonSchema, const MAX: usize> schemars_v0_8::JsonSchema
    for BoundedVec<T, MAX>
{
    fn is_referenceable() -> bool {
        false
    }

    fn schema_name() -> String {
        format!("BoundedVec<{}, {MAX}>", T::schema_name())
    }

    fn json_schema(
        r#gen: &mut schemars_v0_8::r#gen::SchemaGenerator,
    ) -> schemars_v0_8::schema::Schema {
        let mut schema = Vec::<T>::json_schema(r#gen);
        if let schemars_v0_8::schema::Schema::Object(obj) = &mut schema {
            obj.array().max_items = u32::try_from(MAX).ok();
        }
        schema
    }
}

#[cfg(feature = "arbitrary")]
impl<'a, T: arbitrary::Arbitrary<'a>, const MAX: usize> arbitrary::Arbitrary<'a>
    for BoundedVec<T, MAX>
{
    fn arbitrary(u: &mut arbitrary::Unstructured<'a>) -> arbitrary::Result<Self> {
        let mut v = Vec::<T>::arbitrary(u)?;
        v.truncate(MAX);
        Ok(Self(v))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    type Bs8 = BoundedString<8>;
    type Bv3 = BoundedVec<u32, 3>;

    #[test]
    fn string_new_accepts_within_limit() {
        assert_eq!(Bs8::new("hello").unwrap().as_str(), "hello");
        // Exact limit is allowed.
        assert_eq!(Bs8::new("12345678").unwrap().as_str(), "12345678");
        // Empty is allowed.
        assert_eq!(Bs8::new("").unwrap().as_str(), "");
        assert_eq!(Bs8::default().as_str(), "");
        assert_eq!(Bs8::MAX_LEN, 8);
    }

    #[test]
    fn string_new_rejects_oversize() {
        let err = Bs8::new("123456789").unwrap_err();
        assert_eq!(err, TooLongError { len: 9, max: 8 });
        assert_eq!(err.to_string(), "ERR_TOO_LONG: length 9 exceeds the maximum of 8");
        let _: &dyn std::error::Error = &err;
    }

    #[test]
    fn string_bound_is_in_bytes() {
        // "é" is 2 bytes in UTF-8: 4 of them fit, 5 do not.
        assert!(Bs8::new("éééé").is_ok());
        let err = Bs8::new("ééééé").unwrap_err();
        assert_eq!(err.len, 10);
    }

    #[test]
    fn string_conversions() {
        let bs: Bs8 = "abc".parse().unwrap();
        assert_eq!(bs, "abc");
        assert_eq!(bs.to_string(), "abc");
        assert_eq!(bs.len(), 3); // via Deref<Target = str>
        assert_eq!(AsRef::<str>::as_ref(&bs), "abc");
        assert_eq!(AsRef::<[u8]>::as_ref(&bs), b"abc");
        assert_eq!(Bs8::try_from(String::from("abc")).unwrap(), bs);
        assert_eq!(Bs8::try_from("abc").unwrap(), bs);
        assert!(Bs8::try_from("123456789").is_err());
        assert!("123456789".parse::<Bs8>().is_err());
        assert_eq!(String::from(bs.clone()), "abc");
        assert_eq!(bs.into_inner(), "abc");
    }

    #[cfg(feature = "serde")]
    #[test]
    fn string_json_roundtrips_and_enforces_limit() {
        let bs = Bs8::new("hi").unwrap();
        let json = serde_json::to_string(&bs).unwrap();
        assert_eq!(json, "\"hi\"");
        let parsed: Bs8 = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, bs);

        // Oversize rejected at deserialization.
        let err = serde_json::from_str::<Bs8>("\"123456789\"").unwrap_err();
        assert!(err.to_string().contains("ERR_TOO_LONG"), "{err}");
    }

    #[cfg(feature = "borsh")]
    #[test]
    fn string_borsh_roundtrips_and_enforces_limit() {
        let bs = Bs8::new("hi").unwrap();
        let bytes = borsh::to_vec(&bs).unwrap();
        // Wire format matches the plain `String`.
        assert_eq!(bytes, borsh::to_vec(&String::from("hi")).unwrap());
        let parsed: Bs8 = borsh::from_slice(&bytes).unwrap();
        assert_eq!(parsed, bs);

        // Manually craft a borsh-encoded oversized string and confirm it's rejected.
        let bytes = borsh::to_vec(&String::from("123456789")).unwrap();
        let err = borsh::from_slice::<Bs8>(&bytes).unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::InvalidData);
        assert!(err.to_string().contains("ERR_TOO_LONG"), "{err}");
    }

    #[cfg(feature = "schemars-v0_8")]
    #[test]
    fn string_json_schema_has_max_length() {
        let schema = schemars_v0_8::schema_for!(Bs8);
        let string = schema.schema.string.expect("string validation");
        assert_eq!(string.max_length, Some(8));
        assert_eq!(
            schema.schema.instance_type,
            Some(schemars_v0_8::schema::InstanceType::String.into())
        );
    }

    #[cfg(feature = "abi")]
    #[test]
    fn string_borsh_schema_matches_string() {
        use borsh::BorshSchema;
        assert_eq!(Bs8::declaration(), String::declaration());
    }

    #[test]
    fn vec_new_accepts_within_limit() {
        assert_eq!(Bv3::new(vec![1, 2]).unwrap().as_slice(), &[1, 2]);
        // Exact limit is allowed.
        assert_eq!(Bv3::new(vec![1, 2, 3]).unwrap().as_slice(), &[1, 2, 3]);
        // Empty is allowed.
        assert!(Bv3::new(Vec::new()).unwrap().is_empty());
        assert!(Bv3::default().is_empty());
        assert_eq!(Bv3::MAX_LEN, 3);
        // `Default` does not require `T: Default`.
        struct NoDefault;
        let _ = BoundedVec::<NoDefault, 1>::default();
    }

    #[test]
    fn vec_new_rejects_oversize() {
        let err = Bv3::new(vec![1, 2, 3, 4]).unwrap_err();
        assert_eq!(err, TooLongError { len: 4, max: 3 });
    }

    #[test]
    fn vec_conversions_and_mutation() {
        let mut bv = Bv3::try_from(vec![1, 2]).unwrap();
        assert_eq!(bv, [1, 2][..]);
        assert_eq!(bv, vec![1, 2]);
        assert_eq!(bv.len(), 2); // via Deref<Target = [T]>
        assert_eq!(bv.iter().sum::<u32>(), 3);
        assert_eq!((&bv).into_iter().count(), 2);

        bv.try_push(3).unwrap();
        let (err, rejected) = bv.try_push(4).unwrap_err();
        assert_eq!(rejected, 4);
        assert_eq!(err, TooLongError { len: 4, max: 3 });
        assert_eq!(bv.as_slice(), &[1, 2, 3]);

        assert_eq!(bv.pop(), Some(3));
        bv.clear();
        assert!(bv.is_empty());

        let bv = Bv3::new([7, 8]).unwrap();
        assert_eq!(Vec::from(bv.clone()), vec![7, 8]);
        assert_eq!(bv.clone().into_inner(), vec![7, 8]);
        assert_eq!(bv.into_iter().collect::<Vec<_>>(), vec![7, 8]);
    }

    #[cfg(feature = "serde")]
    #[test]
    fn vec_json_roundtrips_and_enforces_limit() {
        let bv = Bv3::new(vec![1, 2]).unwrap();
        let json = serde_json::to_string(&bv).unwrap();
        assert_eq!(json, "[1,2]");
        let parsed: Bv3 = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, bv);

        let err = serde_json::from_str::<Bv3>("[1,2,3,4]").unwrap_err();
        assert!(err.to_string().contains("ERR_TOO_LONG"), "{err}");
    }

    #[cfg(feature = "borsh")]
    #[test]
    fn vec_borsh_roundtrips_and_enforces_limit() {
        let bv = Bv3::new(vec![1, 2]).unwrap();
        let bytes = borsh::to_vec(&bv).unwrap();
        assert_eq!(bytes, borsh::to_vec(&vec![1u32, 2]).unwrap());
        let parsed: Bv3 = borsh::from_slice(&bytes).unwrap();
        assert_eq!(parsed, bv);

        let bytes = borsh::to_vec(&vec![1u32, 2, 3, 4]).unwrap();
        let err = borsh::from_slice::<Bv3>(&bytes).unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::InvalidData);
        assert!(err.to_string().contains("ERR_TOO_LONG"), "{err}");
    }

    #[cfg(feature = "schemars-v0_8")]
    #[test]
    fn vec_json_schema_has_max_items() {
        let schema = schemars_v0_8::schema_for!(Bv3);
        let array = schema.schema.array.expect("array validation");
        assert_eq!(array.max_items, Some(3));
        assert_eq!(
            schema.schema.instance_type,
            Some(schemars_v0_8::schema::InstanceType::Array.into())
        );
    }

    #[cfg(feature = "abi")]
    #[test]
    fn vec_borsh_schema_matches_vec() {
        use borsh::BorshSchema;
        assert_eq!(Bv3::declaration(), Vec::<u32>::declaration());
    }

    #[cfg(feature = "arbitrary")]
    #[test]
    fn arbitrary_values_respect_bounds() {
        use arbitrary::{Arbitrary, Unstructured};
        let raw: Vec<u8> = (0..=255).cycle().take(4096).collect();
        let mut u = Unstructured::new(&raw);
        for _ in 0..64 {
            let s = Bs8::arbitrary(&mut u).unwrap();
            assert!(s.len() <= 8);
            let v = Bv3::arbitrary(&mut u).unwrap();
            assert!(v.len() <= 3);
        }
    }
}
