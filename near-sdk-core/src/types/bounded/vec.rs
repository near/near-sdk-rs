use std::ops::Deref;

use super::{TooLongError, check_len};

/// Vector whose element count is bounded by `MAX`.
///
/// The bound is enforced on construction ([`BoundedVec::new`], [`TryFrom`]) and on Borsh
/// deserialization. Oversized inputs fail with [`TooLongError`].
///
/// `BoundedVec` serializes to a JSON array but does not deserialize from JSON. For bytes passed as
/// JSON use [`BoundedBase64VecU8`](super::BoundedBase64VecU8), which converts to and from
/// `BoundedVec<u8, MAX>`.
///
/// Only the number of elements is bounded. If the elements themselves are variable-size, bound
/// them too (for example `BoundedVec<BoundedString<64>, 8>` in Borsh state or arguments).
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
        check_len(v.len(), MAX)?;
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

#[cfg(feature = "borsh")]
impl<T: borsh::BorshSerialize, const MAX: usize> borsh::BorshSerialize for BoundedVec<T, MAX> {
    fn serialize<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()> {
        self.0.serialize(writer)
    }
}

#[cfg(feature = "borsh")]
impl<T: borsh::BorshDeserialize, const MAX: usize> borsh::BorshDeserialize for BoundedVec<T, MAX> {
    fn deserialize_reader<R: std::io::Read>(reader: &mut R) -> std::io::Result<Self> {
        let mut prefix = [0u8; 4];
        reader.read_exact(&mut prefix)?;
        let len = usize::try_from(u32::from_le_bytes(prefix)).unwrap_or(usize::MAX);
        check_len(len, MAX)?;

        // BorshDeserialize for Vec<T> already checks if type `T` is of size 0 and throws error in
        // this case.
        Vec::<T>::deserialize_reader(&mut std::io::Read::chain(prefix.as_slice(), reader)).map(Self)
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

    type Bv3 = BoundedVec<u32, 3>;

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
    fn vec_json_serializes_as_array() {
        let bv = Bv3::new(vec![1, 2, 3]).unwrap();
        assert_eq!(serde_json::to_string(&bv).unwrap(), "[1,2,3]");
        assert_eq!(serde_json::to_string(&Bv3::default()).unwrap(), "[]");
    }

    #[cfg(feature = "borsh")]
    #[test]
    fn vec_borsh_roundtrips_and_enforces_limit() {
        let bv = Bv3::new(vec![1, 2]).unwrap();
        let bytes = borsh::to_vec(&bv).unwrap();
        assert_eq!(bytes, borsh::to_vec(&vec![1u32, 2]).unwrap());
        let parsed: Bv3 = borsh::from_slice(&bytes).unwrap();
        assert_eq!(parsed, bv);

        let bv = Bv3::new(vec![1, 2, 3]).unwrap();
        let bytes = borsh::to_vec(&bv).unwrap();
        assert_eq!(bytes, borsh::to_vec(&vec![1u32, 2, 3]).unwrap());
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
    fn vec_arbitrary_respects_bound() {
        use arbitrary::{Arbitrary, Unstructured};
        let raw: Vec<u8> = (0..=255).cycle().take(4096).collect();
        let mut u = Unstructured::new(&raw);
        for _ in 0..64 {
            let v = Bv3::arbitrary(&mut u).unwrap();
            assert!(v.len() <= 3);
        }
    }
}
