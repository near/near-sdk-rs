//! How values are stored: leaves (opaque Borsh), records (tagged), and collections
//! (a subtree, nothing inline).

use super::key::Path;
use super::record::RecordError;
use borsh::{BorshDeserialize, BorshSerialize};

/// Anything that can sit at a storage key or in a record field.
///
/// Implemented only on purpose (T7); there is no blanket impl over Borsh:
/// - opaque leaves, written as their Borsh bytes: every [`BorshLeaf`] (primitives, `bool`,
///   `()`, `String`, `AccountId`, `Vec<T>`, `Option<T>`, arrays and tuples of leaves, a few
///   `near_sdk` types, and any struct or enum with `#[derive(Leaf)]`);
/// - `#[near(storage)]` structs get a generated impl: a tagged record;
/// - collections and `Lazy` implement it with `INLINE = false`: they write nothing into
///   their parent record and live under `parent_path ++ tag`.
///
/// v1 collections (`near_sdk::store::*`, `near_sdk::collections::*`) implement none of these,
/// so one inside a record does not compile. They are allowed only as root fields marked
/// `#[key(n, legacy)]`, which are stored as their Borsh bytes without this trait.
///
/// ```compile_fail,E0277
/// use near_sdk::near;
/// use near_sdk::state::*;
/// use near_sdk::store::LookupMap; // shadows `state::LookupMap` from the glob
///
/// #[near(storage)]
/// pub struct Profile {
///     // error[E0277]: `near_sdk::store::LookupMap<u32, u32>` is not a storage v2 value
///     #[key(1)] notes: LookupMap<u32, u32>,
/// }
/// ```
///
/// The same holds for a standards type that holds a v1 collection, such as
/// `near_contract_standards::fungible_token::FungibleToken` (its `accounts` is a
/// `collections::LookupMap`), and for any plain Borsh struct that has not opted in:
///
/// ```compile_fail,E0277
/// use near_sdk::near;
/// use near_sdk::state::Map;
///
/// #[near(serializers = [borsh])]
/// pub struct Token {
///     accounts: near_sdk::collections::LookupMap<near_sdk::AccountId, u128>,
/// }
///
/// #[near(storage)]
/// pub struct Pool {
///     // error[E0277]: `Token` is not a storage v2 value
///     #[key(1, required)] token: Token,
///     #[key(2)] shares: Map<u32, u128>,
/// }
/// ```
///
/// A plain Borsh struct opts in with `#[derive(Leaf)]`:
///
/// ```
/// use near_sdk::near;
/// use near_sdk::state::{Leaf, Map};
///
/// #[near(serializers = [borsh])]
/// #[derive(Leaf, Clone)]
/// pub struct Order { pub price: u128, pub qty: u128 }
///
/// #[near(storage)]
/// pub struct Market {
///     #[key(1)] orders: Map<u64, Order>,
/// }
/// ```
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not a storage v2 value",
    label = "not a `near_sdk::state::Value`",
    note = "a field of a `#[near(storage)]` record or v2 root must be a primitive, `String`, `AccountId`, `Vec`/`Option`/tuple of those, a `near_sdk::state` collection, or a `#[near(storage)]` record; a plain Borsh struct opts in with `#[derive(near_sdk::state::Leaf)]`",
    note = "v1 collections (`near_sdk::store::*`, `near_sdk::collections::*`, and standards types holding one, like `FungibleToken`) are not values: they are allowed only as root fields marked `#[key(n, legacy)]`; inside a record use the `near_sdk::state` type (check for a `use near_sdk::store::..` shadowing a glob import)"
)]
pub trait Value: Sized + 'static {
    /// Whether this value is stored inline in its parent record.
    const INLINE: bool = true;
    /// Worst case number of bytes this value's subtree appends to the path of the key it is
    /// stored at. Zero exactly when the value owns no storage of its own. Summed along the
    /// schema so the root can refuse one that could exceed the 2,048-byte protocol limit.
    const MAX_SUBPATH: usize = 0;
    /// The kind marker byte ([`record::marker`](super::record::marker)) a non-inline value
    /// (a collection or `Lazy`) leaves under its tag in its parent record (API round 5). Unused
    /// for inline values.
    #[doc(hidden)]
    const KIND: u8 = 0;
    /// Whether this is a tagged record (`#[near(storage)]`), whose stored bytes removal checks
    /// for unknown collection tags (API round 5, hazard A3).
    #[doc(hidden)]
    const __RECORD: bool = false;

    /// For a tagged record: the first tag in `bytes` (the stored record) that this type does
    /// not know and whose value is a collection kind marker, or a nested record holding one;
    /// as the tag trail from this record down and the marker. Known nested records are checked
    /// by their own type, known collection tags and `reserved(n)` tags are skipped. `None` for
    /// leaves and collections.
    #[doc(hidden)]
    fn __unknown_collection(_bytes: &[u8]) -> Option<(std::vec::Vec<u32>, u8)> {
        None
    }

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

/// An opaque leaf: a value stored as its Borsh bytes that owns no storage (T7).
///
/// The opt-in to [`Value`] (and to [`Leaf`], [`Removable`] and the view traits) for Borsh
/// types. Implemented for the integers, `bool`, `()`, `String`, `AccountId`, `Vec<T>`,
/// `Option<T>`, `[T; N]`, `BTreeMap`/`BTreeSet` and tuples up to four of leaves, and
/// `NearToken`, `Gas`, `PublicKey` and the `json_types` integers and `Base64VecU8`. A user type
/// opts in with `#[derive(Leaf)]` (from `near_sdk::state`), which emits this impl.
///
/// Not implemented for v1 collections or anything holding one: they own storage under a
/// prefix, which a leaf cannot clear or move, so a record holding one is a compile error.
/// (A hand-written `impl BorshLeaf` or `#[derive(Leaf)]` on a struct that holds a v1
/// collection gets past this; don't.)
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not a storage v2 value",
    label = "not a `near_sdk::state::Value` (no `BorshLeaf` opt-in)",
    note = "a field of a `#[near(storage)]` record or v2 root must be a primitive, `String`, `AccountId`, `Vec`/`Option`/tuple of those, a `near_sdk::state` collection, or a `#[near(storage)]` record; a plain Borsh struct opts in with `#[derive(near_sdk::state::Leaf)]`",
    note = "v1 collections (`near_sdk::store::*`, `near_sdk::collections::*`, and standards types holding one, like `FungibleToken`) are not values: they are allowed only as root fields marked `#[key(n, legacy)]`; inside a record use the `near_sdk::state` type (check for a `use near_sdk::store::..` shadowing a glob import)"
)]
pub trait BorshLeaf: BorshSerialize + BorshDeserialize + 'static {}

impl<T: BorshLeaf> Value for T {
    fn encode_value(&self, _orig: Option<&[u8]>) -> Vec<u8> {
        borsh::to_vec(self).expect("borsh serialization cannot fail")
    }

    fn decode_value(bytes: &[u8], _path: &Path) -> Result<Self, RecordError> {
        borsh::from_slice(bytes).map_err(|e| RecordError::Leaf(e.to_string()))
    }
}

macro_rules! borsh_leaf {
    ($($t:ty),* $(,)?) => { $(impl BorshLeaf for $t {})* };
}
borsh_leaf!(u8, u16, u32, u64, u128, i8, i16, i32, i64, i128, bool, (), String);
borsh_leaf!(crate::AccountId, crate::NearToken, crate::Gas, crate::PublicKey);
borsh_leaf!(
    crate::json_types::U64,
    crate::json_types::U128,
    crate::json_types::I64,
    crate::json_types::I128,
    crate::json_types::Base64VecU8,
);
impl<T: BorshLeaf> BorshLeaf for Vec<T> {}
impl<T: BorshLeaf> BorshLeaf for Option<T> {}
impl<T: BorshLeaf, const N: usize> BorshLeaf for [T; N] {}
impl<K: BorshLeaf + Ord, V: BorshLeaf> BorshLeaf for std::collections::BTreeMap<K, V> {}
impl<T: BorshLeaf + Ord> BorshLeaf for std::collections::BTreeSet<T> {}
impl<A: BorshLeaf, B: BorshLeaf> BorshLeaf for (A, B) {}
impl<A: BorshLeaf, B: BorshLeaf, C: BorshLeaf> BorshLeaf for (A, B, C) {}
impl<A: BorshLeaf, B: BorshLeaf, C: BorshLeaf, D: BorshLeaf> BorshLeaf for (A, B, C, D) {}

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

/// A value that owns no storage below its own key: every [`BorshLeaf`] (blanket impl), and every
/// `#[near(storage)]` record whose fields are all `Leaf` (derived). Collections and `Lazy` are
/// not, and neither is a record holding one.
///
/// `Map::insert` / `LookupMap::insert` always overwrite, so they require `V: Leaf`: overwriting
/// a record that owns collections would leave its old entries behind or silently merge them.
/// For such a record the ways in are `try_insert(k, v)` (`Err(Occupied)` if present) and
/// `entry(k).or_default()`.
#[diagnostic::on_unimplemented(
    message = "`{Self}` owns storage (a collection, a `Lazy`, or a record holding one), so `insert` cannot overwrite it",
    label = "`insert` always overwrites; it needs a value that owns no collections",
    note = "use `try_insert(k, v)` (`Err(Occupied)` if the key exists, else the bound record) or `entry(k).or_default()`; to replace a record, `remove` it first"
)]
pub trait Leaf: Value {}

impl<T: BorshLeaf> Leaf for T {}

/// A value that can be removed from a map without orphaning storage.
///
/// Automatic for every [`BorshLeaf`] (blanket impl), implemented for the iterable collections,
/// and derived for records whose fields are all `Removable`. `clear_children` removes every
/// key below `path` that this value owns, without reading the value itself.
///
/// Not removable: `LookupMap`, `LookupSet` and `GroupedMap` (they do not track their keys), and
/// any record holding one. v1 collections are not values at all (T7), so no record holding one
/// compiles and `remove` can never orphan one.
///
/// # Removal fails closed on unknown collection tags (API round 5, hazard A3)
///
/// `clear_children` clears the collections the CURRENT schema declares. A collection field
/// declared by another version of the contract (a rollback target that predates it, or a later
/// version rolled back from) is not in that schema, so its keys would be left behind and a new
/// record at the same key would inherit them. Every collection or `Lazy` field therefore leaves
/// a one-byte kind marker under its tag in the record (`tag 01 kind`, see
/// [`record::marker`](super::record::marker)), and every removal of a record (`remove`,
/// `discard`, `replace`, `remove_step`, a bulk `clear` of a map, `Vector` or `Lazy` holding
/// records) checks the stored bytes first: a tag the record does not declare, is not listed in
/// `reserved(n)`, and whose value is a kind marker (or a nested record holding one) makes the
/// removal panic, naming the tag, before anything is removed. Roll forward to the version that
/// declares the tag, or list it in `reserved(n)`: a reserved tag is known-and-empty, a
/// statement that the version that owned the collection cleared it. Unknown scalar tags are
/// unaffected (dropped with the record, as before). A one-byte `u8`/`i8` scalar left under an
/// unknown tag whose value is `0x81..=0x87` looks like a marker and fails closed the same way.
///
/// The check reads the record; `remove`, `replace` and `remove_step` read it anyway, and
/// `discard` and bulk clears now pay 1 read per record. Not covered: a blind `insert` of a
/// `Leaf` record (or `Lazy::set` of one) over stored bytes that carry an unknown collection tag
/// replaces them without reading, and records written before round 5 carry no markers until
/// they are next rewritten (`get_mut` or `entry` rewrite them; `at_mut` does not).
#[diagnostic::on_unimplemented(
    message = "`{Self}` cannot be removed or cleared: it is or holds a `LookupMap`/`LookupSet`, which does not track its keys",
    label = "not `Removable`",
    note = "use `Map`/`Set` for collections inside records you remove, or keep the record and clear its fields by hand"
)]
pub trait Removable: Value {
    fn clear_children(_path: &Path) {}

    /// Bounded `clear_children` (D9): removes keys below `path` costing at most `*budget`
    /// `storage_remove` host calls (API round 5; before, `budget` counted map entries and
    /// `Lazy` values were free), from the end, in whole units (a map entry is 3), decrementing
    /// `budget` by each unit's removes. Returns `true` once nothing remains. Resumable across
    /// calls: each collection's header length says where it stopped, and a `Lazy` is checked
    /// for existence before its remove is charged. The default (a value owning nothing) is
    /// done at once.
    fn clear_children_some(path: &Path, _budget: &mut u32) -> bool {
        Self::clear_children(path);
        true
    }
}

impl<T: BorshLeaf> Removable for T {}

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

impl<'a, R: Value, T: BorshLeaf> ViewField<'a, R> for T {
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

impl<'a, R: Value, T: BorshLeaf> ViewFieldMut<'a, R> for T {
    type Out = UseGetMutForScalars;
    fn __view_field_mut(_field_path: Path) -> UseGetMutForScalars {
        UseGetMutForScalars
    }
}
