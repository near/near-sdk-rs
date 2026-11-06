use std::ops::Deref;

use super::{BoundedVec, TooLongError};

/// Bytes whose length is bounded by `MAX`, serialized to JSON as a base64 string.
///
/// Bounded counterpart of [`Base64VecU8`](crate::json_types::Base64VecU8). The bound is enforced
/// on construction ([`BoundedBase64VecU8::new`], [`TryFrom`]) and on both JSON and Borsh
/// deserialization. Oversized inputs fail with [`TooLongError`].
///
/// `MAX` counts decoded bytes. JSON input is standard base64 with padding, and its length is
/// checked before decoding, so an oversized string is never decoded. Like [`BoundedString`], the
/// string is still scanned to the closing quote before the check runs.
///
/// The Borsh wire format is identical to `Vec<u8>`, and the type converts to and from
/// `BoundedVec<u8, MAX>` for free.
///
/// [`BoundedString`]: super::BoundedString
///
/// # Example
/// ```rust
/// use near_sdk::serde_json;
/// use near_sdk_core::types::BoundedBase64VecU8;
///
/// type Payload = BoundedBase64VecU8<4>;
///
/// let payload = Payload::new(vec![1, 2, 3]).unwrap();
/// assert_eq!(serde_json::to_string(&payload).unwrap(), r#""AQID""#);
///
/// let parsed: Payload = serde_json::from_str(r#""AQIDBA==""#).unwrap();
/// assert_eq!(parsed.as_slice(), &[1, 2, 3, 4]);
///
/// assert!(Payload::new(vec![1, 2, 3, 4, 5]).is_err());
/// assert!(serde_json::from_str::<Payload>(r#""AQIDBAU=""#).is_err());
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "borsh", derive(borsh::BorshSerialize, borsh::BorshDeserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
pub struct BoundedBase64VecU8<const MAX: usize>(BoundedVec<u8, MAX>);

impl<const MAX: usize> BoundedBase64VecU8<MAX> {
    /// Maximum number of bytes accepted by this type.
    pub const MAX_LEN: usize = MAX;

    /// Length of the padded base64 encoding of `MAX` bytes, saturated at `usize::MAX`.
    #[cfg(any(feature = "serde", feature = "schemars-v0_8"))]
    const MAX_ENCODED_LEN: usize = match MAX.div_ceil(3).checked_mul(4) {
        Some(len) => len,
        None => usize::MAX,
    };

    /// Wraps `v`, failing with [`TooLongError`] if it holds more than `MAX` bytes.
    pub fn new(v: impl Into<Vec<u8>>) -> Result<Self, TooLongError> {
        BoundedVec::new(v).map(Self)
    }

    /// Returns the wrapped bytes as a slice.
    pub fn as_slice(&self) -> &[u8] {
        self.0.as_slice()
    }

    /// Unwraps into the inner [`Vec`].
    pub fn into_inner(self) -> Vec<u8> {
        self.0.into_inner()
    }
}

impl<const MAX: usize> Deref for BoundedBase64VecU8<MAX> {
    type Target = [u8];

    fn deref(&self) -> &[u8] {
        self.as_slice()
    }
}

impl<const MAX: usize> AsRef<[u8]> for BoundedBase64VecU8<MAX> {
    fn as_ref(&self) -> &[u8] {
        self.as_slice()
    }
}

impl<const MAX: usize> From<BoundedBase64VecU8<MAX>> for Vec<u8> {
    fn from(value: BoundedBase64VecU8<MAX>) -> Self {
        value.into_inner()
    }
}

impl<const MAX: usize> TryFrom<Vec<u8>> for BoundedBase64VecU8<MAX> {
    type Error = TooLongError;

    fn try_from(v: Vec<u8>) -> Result<Self, Self::Error> {
        Self::new(v)
    }
}

impl<const MAX: usize> From<BoundedVec<u8, MAX>> for BoundedBase64VecU8<MAX> {
    fn from(value: BoundedVec<u8, MAX>) -> Self {
        Self(value)
    }
}

impl<const MAX: usize> From<BoundedBase64VecU8<MAX>> for BoundedVec<u8, MAX> {
    fn from(value: BoundedBase64VecU8<MAX>) -> Self {
        value.0
    }
}

impl<const MAX: usize> IntoIterator for BoundedBase64VecU8<MAX> {
    type Item = u8;
    type IntoIter = std::vec::IntoIter<u8>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<'a, const MAX: usize> IntoIterator for &'a BoundedBase64VecU8<MAX> {
    type Item = &'a u8;
    type IntoIter = std::slice::Iter<'a, u8>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

impl<const MAX: usize> PartialEq<[u8]> for BoundedBase64VecU8<MAX> {
    fn eq(&self, other: &[u8]) -> bool {
        self.as_slice() == other
    }
}

impl<const MAX: usize> PartialEq<Vec<u8>> for BoundedBase64VecU8<MAX> {
    fn eq(&self, other: &Vec<u8>) -> bool {
        self.as_slice() == other.as_slice()
    }
}

#[cfg(feature = "serde")]
impl<const MAX: usize> serde::Serialize for BoundedBase64VecU8<MAX> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use base64::Engine;

        serializer.serialize_str(&base64::engine::general_purpose::STANDARD.encode(self.as_slice()))
    }
}

/// The bound is checked on the length of the encoded string before it is decoded, so an oversized
/// string is rejected without decoding it, and decoding allocates at most `MAX + 2` bytes.
#[cfg(feature = "serde")]
impl<'de, const MAX: usize> serde::Deserialize<'de> for BoundedBase64VecU8<MAX> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct Visitor<const MAX: usize>;

        impl<'de, const MAX: usize> serde::de::Visitor<'de> for Visitor<MAX> {
            type Value = BoundedBase64VecU8<MAX>;

            fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "a base64 string of at most {MAX} decoded bytes")
            }

            fn visit_str<E: serde::de::Error>(self, s: &str) -> Result<Self::Value, E> {
                self.visit_bytes(s.as_bytes())
            }

            /// Base64 text handed over as bytes, as `String` also accepts. `visit_byte_buf`
            /// forwards here by default. Bytes that are not ASCII fail base64 decoding.
            fn visit_bytes<E: serde::de::Error>(self, v: &[u8]) -> Result<Self::Value, E> {
                use base64::Engine;

                if v.len() > BoundedBase64VecU8::<MAX>::MAX_ENCODED_LEN {
                    // Decoded length of `v`, assuming it is valid padded base64. It is always
                    // above `MAX` here.
                    let padding = v.iter().rev().take(2).take_while(|&&b| b == b'=').count();
                    let len = v.len().div_ceil(4) * 3 - padding;
                    return Err(E::custom(TooLongError { len, max: MAX }));
                }

                let bytes =
                    base64::engine::general_purpose::STANDARD.decode(v).map_err(E::custom)?;
                // Up to 2 bytes over `MAX` still fit into `MAX_ENCODED_LEN` characters.
                BoundedBase64VecU8::new(bytes).map_err(E::custom)
            }
        }

        deserializer.deserialize_str(Visitor::<MAX>)
    }
}

/// Borsh wire format is identical to `Vec<u8>`; only the accepted length differs.
#[cfg(feature = "abi")]
impl<const MAX: usize> borsh::BorshSchema for BoundedBase64VecU8<MAX> {
    fn add_definitions_recursively(
        definitions: &mut std::collections::BTreeMap<
            borsh::schema::Declaration,
            borsh::schema::Definition,
        >,
    ) {
        Vec::<u8>::add_definitions_recursively(definitions);
    }

    fn declaration() -> borsh::schema::Declaration {
        Vec::<u8>::declaration()
    }
}

/// JSON schema is a `string` with `maxLength` set to the length of the base64 encoding of `MAX`
/// bytes.
///
/// The advertised limit is an upper bound: every accepted value satisfies it, but a value that
/// satisfies it may still decode to up to 2 bytes more than `MAX` and be rejected.
#[cfg(feature = "schemars-v0_8")]
impl<const MAX: usize> schemars_v0_8::JsonSchema for BoundedBase64VecU8<MAX> {
    fn is_referenceable() -> bool {
        false
    }

    fn schema_name() -> String {
        format!("BoundedBase64VecU8<{MAX}>")
    }

    fn json_schema(
        r#gen: &mut schemars_v0_8::r#gen::SchemaGenerator,
    ) -> schemars_v0_8::schema::Schema {
        let mut schema = String::json_schema(r#gen);
        if let schemars_v0_8::schema::Schema::Object(obj) = &mut schema {
            obj.string().max_length = u32::try_from(Self::MAX_ENCODED_LEN).ok();
        }
        schema
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    type Bb4 = BoundedBase64VecU8<4>;

    #[test]
    fn base64_new_accepts_within_limit() {
        assert_eq!(Bb4::new(vec![1, 2]).unwrap().as_slice(), &[1, 2]);
        // Exact limit is allowed.
        assert_eq!(Bb4::new(vec![1, 2, 3, 4]).unwrap().as_slice(), &[1, 2, 3, 4]);
        // Empty is allowed.
        assert!(Bb4::new(Vec::new()).unwrap().is_empty());
        assert!(Bb4::default().is_empty());
        assert_eq!(Bb4::MAX_LEN, 4);
    }

    #[test]
    fn base64_new_rejects_oversize() {
        let err = Bb4::new(vec![1, 2, 3, 4, 5]).unwrap_err();
        assert_eq!(err, TooLongError { len: 5, max: 4 });
    }

    #[test]
    fn base64_conversions() {
        let bb = Bb4::try_from(vec![1, 2]).unwrap();
        assert_eq!(bb.len(), 2); // via Deref<Target = [u8]>
        assert_eq!(AsRef::<[u8]>::as_ref(&bb), &[1, 2]);
        assert!(Bb4::try_from(vec![1, 2, 3, 4, 5]).is_err());
        assert_eq!(Vec::from(bb.clone()), vec![1, 2]);
        assert_eq!(bb.clone().into_inner(), vec![1, 2]);

        assert_eq!(bb, [1, 2][..]);
        assert_eq!(bb, vec![1, 2]);
        assert_eq!((&bb).into_iter().count(), 2);
        assert_eq!(bb.clone().into_iter().collect::<Vec<_>>(), vec![1, 2]);

        let bv = BoundedVec::<u8, 4>::from(bb.clone());
        assert_eq!(bv.as_slice(), &[1, 2]);
        assert_eq!(Bb4::from(bv), bb);
    }

    #[cfg(any(feature = "serde", feature = "schemars-v0_8"))]
    #[test]
    fn base64_max_encoded_len() {
        assert_eq!(BoundedBase64VecU8::<0>::MAX_ENCODED_LEN, 0);
        assert_eq!(BoundedBase64VecU8::<1>::MAX_ENCODED_LEN, 4);
        assert_eq!(BoundedBase64VecU8::<3>::MAX_ENCODED_LEN, 4);
        assert_eq!(Bb4::MAX_ENCODED_LEN, 8);
        assert_eq!(BoundedBase64VecU8::<{ usize::MAX }>::MAX_ENCODED_LEN, usize::MAX);
    }

    #[cfg(feature = "serde")]
    #[test]
    fn base64_json_roundtrips_and_enforces_limit() {
        use crate::json_types::Base64VecU8;

        for bytes in [vec![], vec![1], vec![1, 2, 3], vec![1, 2, 3, 4]] {
            let bb = Bb4::new(bytes.clone()).unwrap();
            let json = serde_json::to_string(&bb).unwrap();
            // Same encoding as the unbounded `Base64VecU8`.
            assert_eq!(json, serde_json::to_string(&Base64VecU8(bytes)).unwrap());
            let parsed: Bb4 = serde_json::from_str(&json).unwrap();
            assert_eq!(parsed, bb);
        }

        // 5 bytes encode to 8 characters, the same as 4 bytes: rejected after decoding.
        let err = serde_json::from_str::<Bb4>(r#""AQIDBAU=""#).unwrap_err();
        assert!(err.to_string().contains("ERR_TOO_LONG: length 5 exceeds"), "{err}");

        // 7 bytes encode to 12 characters: rejected before decoding.
        let err = serde_json::from_str::<Bb4>(r#""AQIDBAUGBw==""#).unwrap_err();
        assert!(err.to_string().contains("ERR_TOO_LONG: length 7 exceeds"), "{err}");

        // `serde_json::Value::String` calls `visit_string`, which forwards to `visit_str`.
        let parsed: Bb4 = serde_json::from_value(serde_json::json!("AQID")).unwrap();
        assert_eq!(parsed.as_slice(), &[1, 2, 3]);
        let err = serde_json::from_value::<Bb4>(serde_json::json!("AQIDBAUGBw==")).unwrap_err();
        assert!(err.to_string().contains("ERR_TOO_LONG"), "{err}");
    }

    #[cfg(feature = "serde")]
    #[test]
    fn base64_json_checks_length_before_decoding() {
        // Invalid base64 within the bound is a decoding error, not a length error.
        let err = serde_json::from_str::<Bb4>(r#""!!!!""#).unwrap_err();
        assert!(err.to_string().contains("Invalid symbol"), "{err}");

        // Oversized and invalid base64: the length check runs first.
        let err = serde_json::from_str::<Bb4>(r#""!!!!!!!!!!!!""#).unwrap_err();
        assert!(err.to_string().contains("ERR_TOO_LONG"), "{err}");

        // Not a string at all.
        let err = serde_json::from_str::<Bb4>("[1,2,3]").unwrap_err();
        assert!(err.to_string().contains("invalid type"), "{err}");
    }

    #[cfg(feature = "serde")]
    #[test]
    fn base64_deserialize_visit_bytes() {
        use serde::Deserialize;
        use serde::de::value::{BytesDeserializer, Error};

        // `BytesDeserializer` calls `visit_bytes`. Without it serde's default rejects bytes with
        // "invalid type: byte array", so a successful parse proves the method exists.
        let ok = Bb4::deserialize(BytesDeserializer::<Error>::new(b"AQIDBA==")).unwrap();
        assert_eq!(ok, vec![1, 2, 3, 4]);

        // Rejected after decoding, then before decoding.
        let err = Bb4::deserialize(BytesDeserializer::<Error>::new(b"AQIDBAU=")).unwrap_err();
        assert!(err.to_string().contains("ERR_TOO_LONG: length 5 exceeds"), "{err}");
        let err = Bb4::deserialize(BytesDeserializer::<Error>::new(b"AQIDBAUGBw==")).unwrap_err();
        assert!(err.to_string().contains("ERR_TOO_LONG: length 7 exceeds"), "{err}");

        // Not ASCII: a base64 decoding error.
        let err = Bb4::deserialize(BytesDeserializer::<Error>::new(&[0xff; 4])).unwrap_err();
        assert!(err.to_string().contains("Invalid symbol"), "{err}");
    }

    #[cfg(feature = "borsh")]
    #[test]
    fn base64_borsh_roundtrips_and_enforces_limit() {
        let bb = Bb4::new(vec![1, 2, 3, 4]).unwrap();
        let bytes = borsh::to_vec(&bb).unwrap();
        // Wire format matches the plain `Vec<u8>`.
        assert_eq!(bytes, borsh::to_vec(&vec![1u8, 2, 3, 4]).unwrap());
        let parsed: Bb4 = borsh::from_slice(&bytes).unwrap();
        assert_eq!(parsed, bb);

        let bytes = borsh::to_vec(&vec![1u8, 2, 3, 4, 5]).unwrap();
        let err = borsh::from_slice::<Bb4>(&bytes).unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::InvalidData);
        assert!(err.to_string().contains("ERR_TOO_LONG"), "{err}");
    }

    #[cfg(feature = "schemars-v0_8")]
    #[test]
    fn base64_json_schema_has_max_length() {
        let schema = schemars_v0_8::schema_for!(Bb4);
        let string = schema.schema.string.expect("string validation");
        assert_eq!(string.max_length, Some(8));
        assert_eq!(
            schema.schema.instance_type,
            Some(schemars_v0_8::schema::InstanceType::String.into())
        );
    }

    #[cfg(feature = "abi")]
    #[test]
    fn base64_borsh_schema_matches_vec() {
        use borsh::BorshSchema;
        assert_eq!(Bb4::declaration(), Vec::<u8>::declaration());
    }

    #[cfg(feature = "arbitrary")]
    #[test]
    fn base64_arbitrary_respects_bound() {
        use arbitrary::{Arbitrary, Unstructured};
        let raw: Vec<u8> = (0..=255).cycle().take(4096).collect();
        let mut u = Unstructured::new(&raw);
        for _ in 0..64 {
            let v = Bb4::arbitrary(&mut u).unwrap();
            assert!(v.len() <= 4);
        }
    }
}
