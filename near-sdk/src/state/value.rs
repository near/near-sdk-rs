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
    /// stored at. Zero exactly when the value owns no storage of its own. Summed along the
    /// schema so the root can refuse one that could exceed the 2,048-byte protocol limit.
    const MAX_SUBPATH: usize = 0;

    /// Encodes the value. `orig` is what is currently stored at the same key or field, so a
    /// record can preserve tags it does not know (`None` for a whole-value replacement).
    fn encode_value(&self, orig: Option<&[u8]>) -> Vec<u8>;

    /// Decodes a value stored at `path`, binding any collections inside it to that path.
    fn decode_value(bytes: &[u8], path: &Path) -> Result<Self, RecordError>;

    /// Binds collections inside this value to `path`. No-op for leaves.
    fn bind(&mut self, _path: &Path) {}

    /// Detaches collections inside this value (after `remove`), so touching them panics.
    fn detach(&mut self) {}
}

impl<T: BorshSerialize + BorshDeserialize + 'static> Value for T {
    fn encode_value(&self, _orig: Option<&[u8]>) -> Vec<u8> {
        borsh::to_vec(self).expect("borsh serialization cannot fail")
    }

    fn decode_value(bytes: &[u8], _path: &Path) -> Result<Self, RecordError> {
        borsh::from_slice(bytes).map_err(|e| RecordError::Leaf(e.to_string()))
    }
}

/// The value of a field missing from a stored record.
///
/// Implemented for every `Default` type. A field whose type has no `Default` must be
/// `#[key(n, required)]` or `Option<T>`.
#[diagnostic::on_unimplemented(
    message = "storage field of type `{Self}` has no `Default`, so an old record without this tag could not be decoded",
    label = "write `#[key(n, required)]` on this field, or make it `Option<{Self}>`",
    note = "`required` is only for fields present since the record's first write; a field added after deploy must be `Default` or `Option`"
)]
pub trait FieldDefault: Sized {
    fn field_default() -> Self;
}

impl<T: Default> FieldDefault for T {
    fn field_default() -> Self {
        T::default()
    }
}

/// A `#[near(storage)]` struct: a tagged record that may contain collections.
pub trait Record: Value {
    /// The view `Map::at` returns: the record's collection and `Lazy` fields as handles bound
    /// to their nested paths. Reads nothing.
    type View<'a>;
    #[doc(hidden)]
    fn __view<'a>(path: Path) -> Self::View<'a>;
}

/// A value that can be removed from a map without orphaning storage.
///
/// Automatic for every Borsh leaf (blanket impl), implemented for the iterable collections,
/// and derived for records whose fields are all `Removable`. `clear_children` removes every
/// key below `path` that this value owns, without reading the value itself.
///
/// Caveat: v1 collections are Borsh leaves too, so they are `Removable` by this rule even
/// though they own storage. That is only safe because they are root-only (the derive rejects
/// the common v1 type names in records; a renamed or aliased type slips through).
pub trait Removable: Value {
    fn clear_children(_path: &Path) {}
}

impl<T: BorshSerialize + BorshDeserialize + 'static> Removable for T {}

/// How a field of a record appears in that record's `View`.
///
/// Collections and `Lazy` appear as [`At`](super::At) handles bound to the nested path, whose
/// reads live as long as the view's borrow. Nested records appear as their own view. Scalars
/// appear as `UseGetForScalars` (hidden from docs): `at` walks the path without reading; use `get` to read a
/// record's scalars.
pub trait ViewField<'a, R: Value>: Value {
    type Out;
    #[doc(hidden)]
    fn __view_field(field_path: Path) -> Self::Out;
}

/// The type a scalar field has inside a `View`: nothing can be done with it. `at(&k)` walks the
/// path without reading the record; read scalars with `get(&k)`. The name is the hint: it is
/// what autocomplete shows for `at(&k).nonce`, and `.get()` on it fails with "no method named
/// `get` found for struct `UseGetForScalars`".
#[doc(hidden)]
#[derive(Clone, Copy, Debug)]
pub struct UseGetForScalars;

impl<'a, R: Value, T: BorshSerialize + BorshDeserialize + 'static> ViewField<'a, R> for T {
    type Out = UseGetForScalars;
    fn __view_field(_field_path: Path) -> UseGetForScalars {
        UseGetForScalars
    }
}
