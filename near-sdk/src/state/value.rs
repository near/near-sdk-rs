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

    /// Writes `Lazy::new` values held by this value (or its inline nested records) at
    /// `path`, without binding. Called on the root by the flush, so a root built as a struct
    /// literal in `#[init]` writes its pending `Lazy` values without a `.bind()` call.
    #[doc(hidden)]
    fn __write_pending(&self, _path: &Path) {}
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
    /// `{Name}At<'a>`, returned by `at(&k)`: the record's collection and `Lazy` fields as
    /// read-only handles bound to their nested paths. Reads nothing.
    type View<'a>;
    /// `{Name}AtMut<'a>`, returned by `at_mut(&k)`: the record's collection and `Lazy` fields
    /// as `&'a mut` handles. Reads nothing below the record.
    type ViewMut<'a>;
    #[doc(hidden)]
    fn __view<'a>(path: Path) -> Self::View<'a>;
    #[doc(hidden)]
    fn __view_mut<'a>(path: Path) -> Self::ViewMut<'a>;
}

/// A value that owns no storage below its own key: every Borsh type (blanket impl), and every
/// `#[near(storage)]` record whose fields are all `Leaf` (derived). Collections and `Lazy` are
/// not, and neither is a record holding one.
///
/// `Map::insert` / `LookupMap::insert` always overwrite, so they require `V: Leaf`: overwriting
/// a record that owns collections would leave its old entries behind or silently merge them.
/// For such a record the ways in are `insert_new(k, v)` (panics if present) and
/// `entry(k).or_default()`.
#[diagnostic::on_unimplemented(
    message = "`{Self}` owns storage (a collection, a `Lazy`, or a record holding one), so `insert` cannot overwrite it",
    label = "`insert` always overwrites; it needs a value that owns no collections",
    note = "use `insert_new(k, v)` (panics if the key exists, returns the bound record) or `entry(k).or_default()`; to replace a record, `remove` it first"
)]
pub trait Leaf: Value {}

impl<T: BorshSerialize + BorshDeserialize + 'static> Leaf for T {}

/// A value that can be removed from a map without orphaning storage.
///
/// Automatic for every Borsh leaf (blanket impl), implemented for the iterable collections,
/// and derived for records whose fields are all `Removable`. `clear_children` removes every
/// key below `path` that this value owns, without reading the value itself.
///
/// Not removable: `LookupMap`, `LookupSet` and `GroupedMap` (they do not track their keys), and
/// any record holding one.
///
/// Caveat: v1 collections are Borsh leaves too, so they are `Removable` by this rule even
/// though they own storage. v1 collections belong at the root; `remove` does not clear a
/// nested one.
#[diagnostic::on_unimplemented(
    message = "`{Self}` cannot be removed or cleared: it is or holds a `LookupMap`/`LookupSet`, which does not track its keys",
    label = "not `Removable`",
    note = "use `Map`/`Set` for collections inside records you remove, or keep the record and clear its fields by hand"
)]
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

/// How a field of a record appears in that record's `AtMut` view: collections and `Lazy` as
/// `&'a mut` handles, nested records as their own `AtMut`, scalars as `UseGetMutForScalars`.
pub trait ViewFieldMut<'a, R: Value>: Value {
    type Out;
    #[doc(hidden)]
    fn __view_field_mut(field_path: Path) -> Self::Out;
}

/// The type a scalar field has inside an `AtMut` view. `at_mut(&k)` does not read the record,
/// so its scalars are not available; use `get_mut(&k)` to change them.
#[doc(hidden)]
#[derive(Clone, Copy, Debug)]
pub struct UseGetMutForScalars;

impl<'a, R: Value, T: BorshSerialize + BorshDeserialize + 'static> ViewFieldMut<'a, R> for T {
    type Out = UseGetMutForScalars;
    fn __view_field_mut(_field_path: Path) -> UseGetMutForScalars {
        UseGetMutForScalars
    }
}
