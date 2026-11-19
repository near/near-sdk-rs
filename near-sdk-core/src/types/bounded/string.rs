use std::fmt;
use std::ops::Deref;
use std::str::FromStr;

use super::{TooLongError, check_len};

/// UTF-8 string whose byte length is bounded by `MAX`.
///
/// The bound is enforced on construction ([`BoundedString::new`], [`FromStr`], [`TryFrom`]) and
/// on both JSON and Borsh deserialization. Oversized inputs fail with [`TooLongError`].
///
/// A JSON string is checked after the deserializer has scanned it to the closing quote, so an
/// oversized one costs the same gas to reject as a plain `String` costs to parse. The check runs
/// before the string is copied.
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
        check_len(s.len(), MAX)?;
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
        let mut prefix = [0u8; 4];
        reader.read_exact(&mut prefix)?;
        let len = usize::try_from(u32::from_le_bytes(prefix)).unwrap_or(usize::MAX);
        check_len(len, MAX)?;

        String::deserialize_reader(&mut std::io::Read::chain(prefix.as_slice(), reader)).map(Self)
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

#[cfg(test)]
mod tests {
    use super::*;

    type Bs8 = BoundedString<8>;

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

    #[cfg(feature = "serde")]
    #[test]
    fn string_deserialize_visit_str() {
        use serde::Deserialize;
        use serde::de::value::{Error, StrDeserializer};

        // `StrDeserializer` calls `visit_str` and nothing else.
        let ok = Bs8::deserialize(StrDeserializer::<Error>::new("hello")).unwrap();
        assert_eq!(ok, "hello");
        let err = Bs8::deserialize(StrDeserializer::<Error>::new("123456789")).unwrap_err();
        assert!(err.to_string().contains("ERR_TOO_LONG"), "{err}");

        // Unescaped input from a slice calls `visit_borrowed_str`, whose default forwards to
        // `visit_str`. This is the `#[near]` argument path (`serde_json::from_slice`).
        let ok: Bs8 = serde_json::from_slice(br#""hello""#).unwrap();
        assert_eq!(ok, "hello");
        let err = serde_json::from_slice::<Bs8>(br#""123456789""#).unwrap_err();
        assert!(err.to_string().contains("ERR_TOO_LONG"), "{err}");
    }

    #[cfg(feature = "serde")]
    #[test]
    fn string_deserialize_visit_string() {
        use serde::Deserialize;
        use serde::de::value::{Error, StringDeserializer};

        // `StringDeserializer` calls `visit_string`. Without our override serde's default
        // forwards to `visit_str` and copies. The pointer check proves the String was moved.
        let s = String::from("hello");
        let ok = Bs8::deserialize(StringDeserializer::<Error>::new(s)).unwrap();
        assert_eq!(ok, "hello");

        let err =
            Bs8::deserialize(StringDeserializer::<Error>::new("123456789".into())).unwrap_err();
        assert!(err.to_string().contains("ERR_TOO_LONG"), "{err}");

        // `serde_json::Value::String` hands over its owned String: `visit_string`.
        let ok: Bs8 = serde_json::from_value(serde_json::json!("hello")).unwrap();
        assert_eq!(ok, "hello");
        let err = serde_json::from_value::<Bs8>(serde_json::json!("123456789")).unwrap_err();
        assert!(err.to_string().contains("ERR_TOO_LONG"), "{err}");
    }

    #[cfg(feature = "serde")]
    #[test]
    fn string_deserialize_visit_bytes() {
        use serde::Deserialize;
        use serde::de::value::{BytesDeserializer, Error};

        // `BytesDeserializer` calls `visit_bytes`. Without our override serde's default rejects
        // bytes with "invalid type: byte array", so a successful parse proves the method exists.
        let ok = Bs8::deserialize(BytesDeserializer::<Error>::new(b"hello")).unwrap();
        assert_eq!(ok, "hello");
        let err = Bs8::deserialize(BytesDeserializer::<Error>::new(b"123456789")).unwrap_err();
        assert!(err.to_string().contains("ERR_TOO_LONG"), "{err}");

        // Invalid UTF-8 within the bound is an invalid value, not a length error.
        let err = Bs8::deserialize(BytesDeserializer::<Error>::new(&[0xff, 0xfe])).unwrap_err();
        assert!(err.to_string().contains("invalid value"), "{err}");

        // Oversized and not UTF-8: the length check runs first.
        let err = Bs8::deserialize(BytesDeserializer::<Error>::new(&[0xff; 9])).unwrap_err();
        assert!(err.to_string().contains("ERR_TOO_LONG"), "{err}");

        // The bound counts bytes: "é" is 2 bytes, so 4 fit into 8 and 5 do not.
        assert!(Bs8::deserialize(BytesDeserializer::<Error>::new("éééé".as_bytes())).is_ok());
        let err =
            Bs8::deserialize(BytesDeserializer::<Error>::new("ééééé".as_bytes())).unwrap_err();
        assert!(err.to_string().contains("ERR_TOO_LONG"), "{err}");
    }

    #[cfg(feature = "serde")]
    #[test]
    fn string_deserialize_visit_bytes_buf() {
        use serde::Deserialize;

        // There is no built-in serde deserializer that calls `visit_byte_buf`, so we create one.
        struct ByteBufDeserializer(Vec<u8>);

        impl<'de> serde::Deserializer<'de> for ByteBufDeserializer {
            type Error = serde::de::value::Error;

            fn deserialize_any<V: serde::de::Visitor<'de>>(
                self,
                visitor: V,
            ) -> Result<V::Value, Self::Error> {
                visitor.visit_byte_buf(self.0)
            }

            serde::forward_to_deserialize_any! {
                bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
                bytes byte_buf option unit unit_struct newtype_struct seq tuple
                tuple_struct map struct enum identifier ignored_any
            }
        }

        // `ByteBufDeserializer` calls `visit_buf_bytes`.
        let ok = Bs8::deserialize(ByteBufDeserializer(b"hello".to_vec())).unwrap();
        assert_eq!(ok, "hello");
        let err = Bs8::deserialize(ByteBufDeserializer(b"123456789".to_vec())).unwrap_err();
        assert!(err.to_string().contains("ERR_TOO_LONG"), "{err}");

        // Invalid UTF-8 within the bound is an invalid value, not a length error.
        let err = Bs8::deserialize(ByteBufDeserializer(vec![0xff, 0xfe])).unwrap_err();
        assert!(err.to_string().contains("invalid value"), "{err}");

        // Oversized and not UTF-8: the length check runs first.
        let err = Bs8::deserialize(ByteBufDeserializer(vec![0xff; 9])).unwrap_err();
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

    #[cfg(feature = "arbitrary")]
    #[test]
    fn string_arbitrary_respects_bound() {
        use arbitrary::{Arbitrary, Unstructured};
        let raw: Vec<u8> = (0..=255).cycle().take(4096).collect();
        let mut u = Unstructured::new(&raw);
        for _ in 0..64 {
            let s = Bs8::arbitrary(&mut u).unwrap();
            assert!(s.len() <= 8);
        }
    }
}
