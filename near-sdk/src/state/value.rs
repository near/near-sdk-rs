//! How values are stored: leaves (opaque Borsh), records (tagged), and collections
//! (a subtree, nothing inline).

use super::key::Path;
use super::record::RecordError;
use borsh::{BorshDeserialize, BorshSerialize};

/// Anything that can sit at a storage key or in a record field.
///
/// - Borsh types get a blanket impl: an opaque leaf, written as its Borsh bytes.
/// - `#[near(storage)]` structs get a generated impl: a tagged record.
/// - Collections and `Lazy` implement it with `INLINE = false`: they write nothing into
///   their parent record and live under `parent_path ++ tag`.
pub trait Value: Sized + 'static {
    /// Whether this value is stored inline in its parent record.
    const INLINE: bool = true;
    /// Worst case number of bytes this value's subtree appends to the path of the key it is
    /// stored at. Summed along the schema so the root can refuse one that could exceed
    /// the 2,048-byte protocol limit.
    const MAX_SUBPATH: usize = 0;

    /// Encodes the value. `orig` is what is currently stored at the same key or field, so a
    /// record can preserve tags it does not know.
    fn encode_value(&self, orig: Option<&[u8]>) -> Vec<u8>;

    /// Decodes a value stored at `path`, binding any collections inside it to that path.
    fn decode_value(bytes: &[u8], path: &Path) -> Result<Self, RecordError>;

    /// Binds collections inside this value to `path`. No-op for leaves.
    fn bind(&mut self, _path: &Path) {}

    /// Detaches collections inside this value, so touching them panics.
    fn unbind(&mut self) {}

    /// The value of a field missing from its record, or of a fresh `or_default()` entry.
    fn missing_at(path: &Path) -> Self
    where
        Self: Default,
    {
        let mut v = Self::default();
        v.bind(path);
        v
    }
}

impl<T: BorshSerialize + BorshDeserialize + 'static> Value for T {
    fn encode_value(&self, _orig: Option<&[u8]>) -> Vec<u8> {
        borsh::to_vec(self).expect("borsh serialization cannot fail")
    }

    fn decode_value(bytes: &[u8], _path: &Path) -> Result<Self, RecordError> {
        borsh::from_slice(bytes).map_err(|e| RecordError::Leaf(e.to_string()))
    }
}

/// A `#[near(storage)]` struct: a tagged record that may contain collections.
pub trait Record: Value {
    /// The view `Map::at` returns: collection handles at nested paths, scalars read lazily.
    type View<'a>;
    #[doc(hidden)]
    fn __view<'a>(path: Path) -> Self::View<'a>;
}

/// A value that can be removed from a map without orphaning storage.
///
/// Implemented for scalars, for iterable collections, and by the derive for records whose
/// fields are all `Removable`. `clear_children` removes every key below `path` that this
/// value owns, without reading the value itself (paths are computable from the schema).
pub trait Removable: Value {
    fn clear_children(_path: &Path) {}
}

/// Opt a plain Borsh leaf into `Removable`, asserting that it holds no storage of its own.
#[macro_export]
#[doc(hidden)]
macro_rules! __state_removable_leaf {
    ($($t:ty),* $(,)?) => {$( impl $crate::state::Removable for $t {} )*};
}
pub use crate::__state_removable_leaf as removable_leaf;

removable_leaf!(
    u8, u16, u32, u64, u128, i8, i16, i32, i64, i128, bool, (), String,
    crate::AccountId, crate::json_types::U128, crate::json_types::U64,
    crate::NearToken, crate::Gas,
);
impl<T: Removable + BorshSerialize + BorshDeserialize> Removable for Vec<T> {}
impl<T: Removable + BorshSerialize + BorshDeserialize> Removable for Option<T> {}
impl<A: Removable + BorshSerialize + BorshDeserialize, B: Removable + BorshSerialize + BorshDeserialize>
    Removable for (A, B)
{
}
impl<const N: usize> Removable for [u8; N] {}

/// How a field of a record appears in that record's `View`.
///
/// Collections appear as handles bound to the nested path (no read). Scalars appear as a
/// [`Scalar`] that reads the parent record once when asked.
pub trait ViewField<'a, R: Value>: Value {
    type Out;
    #[doc(hidden)]
    fn __view_field(record_path: &Path, field_path: Path, project: fn(&R) -> &Self) -> Self::Out;
}

impl<'a, R: Value, T: BorshSerialize + BorshDeserialize + 'static> ViewField<'a, R> for T {
    type Out = Scalar<'a, R, T>;
    fn __view_field(record_path: &Path, _field_path: Path, project: fn(&R) -> &T) -> Scalar<'a, R, T> {
        Scalar { record_path: record_path.clone(), project, _m: core::marker::PhantomData }
    }
}

/// A scalar field reached through a `View`: reading it reads (and caches) the whole record.
pub struct Scalar<'a, R, T> {
    record_path: Path,
    project: fn(&R) -> &T,
    _m: core::marker::PhantomData<&'a R>,
}

impl<'a, R: Value, T> Scalar<'a, R, T> {
    /// 1 read of the parent record (cached). `None` if the record is absent.
    pub fn get(&self) -> Option<&'a T> {
        // SAFETY: 'a is the borrow of the collection the view came from.
        unsafe { super::root::__private::get_record::<R>(&self.record_path) }.map(self.project)
    }
}
