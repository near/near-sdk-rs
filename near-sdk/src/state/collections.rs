//! The v2 collection family. Every handle is a path; every byte goes through the cache.
//!
//! Handles are not `Clone`: two handles to one path would break the cache's aliasing
//! invariant (see `cache.rs`).

use super::cache;
use super::key::{ENTRY_HASH_LEN, Path, StorageKey, kind};
use super::record::RecordError;
use super::value::{Leaf, Record, Removable, Value, ViewField, ViewFieldMut};
use core::marker::PhantomData;
use core::num::NonZeroU32;
use core::ptr::NonNull;

/// SAFETY: see the invariant in `cache.rs`. The returned lifetime is tied by the caller to a
/// borrow of the handle (or view) that owns the key.
unsafe fn r<'a, V>(p: NonNull<V>) -> &'a V {
    unsafe { &*p.as_ptr() }
}
/// SAFETY: as `r`, and the caller holds `&mut` on the owning handle.
unsafe fn m<'a, V>(p: NonNull<V>) -> &'a mut V {
    unsafe { &mut *p.as_ptr() }
}

/// Round 7 (the A3 first-write rule): where a handle reached through `at_mut` sits in the
/// record that stores it. `rec_len` is that record's key length, a prefix of the handle's path
/// (the rest of the path is the field tags leading to the collection); 0 means none: a root
/// field, or a handle bound by decoding its record, which is then Touched or Dirty and
/// re-encoded with every marker at the flush. `kind` is the marker the collection's field
/// leaves (`Set`, not the `Map` inside it).
#[derive(Clone, Copy, PartialEq, Eq, Default)]
struct Anchor {
    rec_len: u32,
    kind: u8,
}

/// Where a collection handle points.
#[derive(Clone, PartialEq, Eq, Default)]
enum Binding {
    /// Constructed with `Default` and not yet placed.
    #[default]
    Unbound,
    /// Bound to a path, stamped with the removal epoch it was last known live at (audit A2,
    /// [`cache::check_live`]): set when bound, advanced by `live()`. The anchor is set only for
    /// handles from `at_mut` views.
    Bound(Path, u64, Anchor),
    /// The record holding this collection was removed; its children were cleared.
    Detached,
}

impl core::fmt::Debug for Binding {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Binding::Unbound => f.write_str("Unbound"),
            Binding::Bound(p, ..) => f.debug_tuple("Bound").field(p).finish(),
            Binding::Detached => f.write_str("Detached"),
        }
    }
}

impl Binding {
    /// A handle bound to `path` now.
    fn at(path: Path) -> Self {
        Binding::Bound(path, cache::epoch(), Anchor::default())
    }

    /// A handle bound to `path` now, inside the record stored at the first `anchor` bytes of
    /// `path` (an `at_mut` view field), whose field leaves the marker `kind`.
    fn anchored(path: Path, anchor: usize, kind: u8) -> Self {
        debug_assert!(anchor > 0 && anchor < path.len(), "an anchor is a proper prefix");
        let rec_len = u32::try_from(anchor).expect("a key is at most 2,048 bytes");
        Binding::Bound(path, cache::epoch(), Anchor { rec_len, kind })
    }

    /// Round 7, the A3 first-write rule. Called by a write that would bring this collection
    /// into existence (a `Map`/`Set`/`Vector` whose header is absent, a `Lazy` set): if the
    /// handle came from `at_mut`, the record storing it must declare it, so its kind marker is
    /// added to the stored record when missing ([`cache::ensure_marker`]). `child_absent` is
    /// asked only when the record's bytes are not known in this call. Free for root fields and
    /// for handles of a decoded record.
    #[track_caller]
    #[inline(never)]
    fn declare(&self, child_absent: &dyn Fn() -> bool) {
        if let Binding::Bound(p, _, a) = self
            && a.rec_len > 0
        {
            let (rec, hops) = p.as_bytes().split_at(a.rec_len as usize);
            let mut trail = std::vec::Vec::new();
            let mut pos = 0;
            while pos < hops.len() {
                match super::record::read_varint_u32(hops, &mut pos) {
                    Ok(t) => trail.push(t),
                    Err(_) => panic!("field hops are LEB128 tags"),
                }
            }
            cache::ensure_marker(rec, &trail, a.kind, child_absent);
        }
    }

    /// Audit A2: before every write through a handle, panics if the record it lives under was
    /// removed (or started `remove_step` clearing) in this call after the handle was bound,
    /// which only an escaped handle (`mem::take` on a field) can see. One thread-local load and
    /// one `u64` compare unless a removal happened since this handle last checked. Unbound and
    /// detached handles fall through to `bound`'s panics.
    #[track_caller]
    fn live(&mut self) {
        if let Binding::Bound(p, seen, _) = self {
            cache::check_live(p, seen);
        }
    }
}

#[track_caller]
fn bound(b: &Binding) -> &Path {
    match b {
        Binding::Bound(p, ..) => p,
        Binding::Unbound => panic!(
            "state: collection is not bound to a path; place its record with `insert` or \
             `or_default()` first (in `#[init]`, call `.bind()` on the root). If the record was \
             moved out with `mem::take`/`mem::replace`, that is unsupported"
        ),
        Binding::Detached => panic!("state: collection detached by remove; read it before removing"),
    }
}

#[track_caller]
fn bind_slot(slot: &mut Binding, path: &Path) {
    match slot {
        Binding::Bound(old, ..) if old != path => panic!(
            "state: collection already bound to {old:?}; a record cannot move to another key \
             (`mem::take`/`mem::replace` on a stored record is unsupported; remove it and build \
             a new one)"
        ),
        Binding::Detached => panic!(
            "state: this record was removed and its collections cleared; build a new record \
             instead of re-inserting the removed one"
        ),
        _ => *slot = Binding::at(path.clone()),
    }
}

fn not_inline<T>() -> Result<T, RecordError> {
    Err(RecordError::Leaf("a collection has no inline encoding".into()))
}

/// A `Map`'s header (T4): its length and a revision counter that every removal bumps, so two
/// pages read with no removal in between carry the same `revision`.
///
/// Encoded as the bare little-endian `u32` length while `revision` is 0 (the pre-T4 4-byte
/// header, so a map that never had a removal is byte-identical), and as `len ++ revision`
/// (12 bytes, Borsh `(u32, u64)`) afterwards. The header of a map that empties after a removal
/// stays (`len` 0) so the revision never resets; it is removed only when the record owning the
/// map is removed or cleared (`Removable::clear_children[_some]`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct MapHeader {
    len: u32,
    revision: u64,
}

impl Value for MapHeader {
    fn encode_value(&self, _orig: Option<&[u8]>) -> std::vec::Vec<u8> {
        let mut b = self.len.to_le_bytes().to_vec();
        if self.revision != 0 {
            b.extend_from_slice(&self.revision.to_le_bytes());
        }
        b
    }
    fn decode_value(bytes: &[u8], _path: &Path) -> Result<Self, RecordError> {
        let u32_at = |b: &[u8]| u32::from_le_bytes(b[..4].try_into().unwrap());
        match bytes.len() {
            4 => Ok(MapHeader { len: u32_at(bytes), revision: 0 }),
            12 => Ok(MapHeader {
                len: u32_at(bytes),
                revision: u64::from_le_bytes(bytes[4..].try_into().unwrap()),
            }),
            n => Err(RecordError::Leaf(format!("map header of {n} bytes (want 4 or 12)"))),
        }
    }
}

/// Writes a collection header. The default header (empty, never removed from) is no key at all.
fn put_header<H: Value + Default + PartialEq>(path: &Path, h: H) {
    if h == H::default() {
        cache::delete::<H>(path);
    } else {
        cache::put(path, h);
    }
}

/// `at_mut`: 1 `storage_has_key` on the record key unless cached, no read, no decode. `None`
/// if the record is absent (so it never creates children of a record that does not exist).
/// A record being cleared (D9) is stored at its flag key with its record key absent, so the
/// one check on a present record also says it is not being cleared; only a miss makes a 2nd
/// check, of the flag key, and panics if the record is being cleared.
#[track_caller]
fn record_at_mut<'a, V: Record>(path: Path) -> Option<V::ViewMut<'a>> {
    let clearing = cache::record_state::<V>(&path)?;
    if clearing && V::MAX_SUBPATH > 0 {
        clearing_panic(&path);
    }
    Some(V::__view_mut(path))
}

#[track_caller]
fn clearing_panic(path: &Path) -> ! {
    panic!(
        "state: the record at {path:?} is being cleared by `remove_step`; writes to it or below \
         it are refused until clearing completes (call `remove_step` again until it returns \
         `Removal::Done`)"
    )
}

/// D9: panics if the record at `path` (already read) is being cleared. Only a value that owns
/// storage can be.
#[track_caller]
fn assert_not_clearing<V: Value>(path: &Path) {
    if V::MAX_SUBPATH > 0 && cache::is_clearing(path) {
        clearing_panic(path);
    }
}

/// The stored value at `path` for writing (1 read): refused if its record is being cleared (D9),
/// touched so the flush writes it if it changed.
///
/// SAFETY: as `m`.
#[track_caller]
unsafe fn get_entry_mut<'a, V: Value>(path: &Path) -> Option<&'a mut V> {
    let p = cache::get_entry::<V>(path)?;
    assert_not_clearing::<V>(path);
    cache::touch(path);
    Some(unsafe { m(p) })
}

/// Moves a bound handle into the per-call arena and returns `&'a mut` to it.
///
/// SAFETY (for callers): `'a` must be bounded by a `&mut` borrow of the parent collection,
/// and the handle is freed at the flush (invariant 4 in `cache.rs`).
unsafe fn arena_mut<'a, C: core::any::Any>(c: C) -> &'a mut C {
    unsafe { &mut *cache::arena(c).as_ptr() }
}

/// API round 5 (hazard A3), "removal fails closed on unknown collection tags": before the
/// record stored at `path` is removed, checks its stored bytes for a tag this code does not
/// declare (and does not list in `reserved(n)`) whose value is a collection kind marker, or a
/// nested record holding one. Such a tag is a collection another version of the contract
/// declared; removing the record would orphan its keys, and a new record at the same key would
/// inherit them. Panics naming the tag. Only for tagged records (`V::__RECORD`); costs 1 read
/// when this call has not read the record yet (`discard`, a bulk clear), nothing otherwise.
#[track_caller]
fn guard_removal<V: Value>(path: &Path, entry: bool) {
    if !V::__RECORD {
        return;
    }
    let Some(bytes) = cache::stored_bytes(path, entry && V::MAX_SUBPATH > 0) else { return };
    if let Some((trail, k)) = V::__unknown_collection(&bytes) {
        unknown_collection_panic(path, &trail, k);
    }
}

#[track_caller]
fn unknown_collection_panic(path: &Path, trail: &[u32], k: u8) -> ! {
    let tag = trail.iter().map(u32::to_string).collect::<std::vec::Vec<_>>().join(" > ");
    let fix = match trail {
        [t] => format!("`#[near(storage, reserved({t}))]`"),
        _ => format!("`reserved(n)` on the record that dropped it (tag path {tag})"),
    };
    crate::env::panic_str(&format!(
        "state: refusing to remove the record at {path:?}: its stored bytes carry tag {tag}, a \
         `{}` field this version of the record does not declare (another version of the \
         contract wrote it). Removing the record would leave that collection's keys behind, \
         and a new record at this key would inherit them. Roll forward to the version that \
         declares tag {tag} and remove the record there, or, if that version cleared the \
         collection before the field was dropped, list the tag in {fix}",
        super::record::marker::name(k)
    ))
}

/// Removes the value just read at `path`, its collections first, and hands it back detached.
/// Panics, changing nothing, on an undeclared collection tag (`guard_removal`).
#[track_caller]
fn take_read<V: Removable>(path: &Path) -> V {
    guard_removal::<V>(path, true);
    V::clear_children(path);
    let mut v = cache::take_entry::<V>(path).expect("value was just read");
    v.detach();
    v
}

/// Panics if `V` owns collections and a value already exists at `path`.
#[track_caller]
fn assert_insert_new<V: Value>(path: &Path) {
    if V::MAX_SUBPATH > 0 && cache::exists(path) {
        panic!("state: record exists; use get_mut, or remove it first");
    }
}

/// Runs `f` (some deletes whose host removes are `cost`) if `cost` fits in `budget`, and
/// charges it. `false`, with nothing done, when it does not fit (API round 5).
fn spend(budget: &mut u32, cost: u32, f: impl FnOnce()) -> bool {
    if cost > *budget {
        return false;
    }
    *budget -= cost;
    f();
    true
}

/// The smallest budget `remove_step` accepts: a `Map` entry is 3 host removes (entry, index,
/// key slot) that cannot be split, since a clearing record stays readable and a half-removed
/// entry would break its map's iteration; finishing a record held in a `Map` is likewise 3 on
/// the parent map (record, index, key slot). With `n >= 3` every call makes progress. A smaller
/// `n` panics instead of returning `Pending` forever.
pub const MIN_REMOVE_STEP: u32 = 3;

#[track_caller]
fn check_step(n: core::num::NonZeroU32) -> u32 {
    let n = n.get();
    if n < MIN_REMOVE_STEP {
        panic!(
            "state: `remove_step(k, {n})`: the budget counts host removes and must be at least \
             {MIN_REMOVE_STEP}, since one map entry is {MIN_REMOVE_STEP} removes (entry, index, \
             key slot) that cannot be split; with less, clearing could never progress"
        );
    }
    n
}

/// `remove_step` on either map, for the record at `path` in `map`, after `check_step`: clears
/// what fits in `n` removes, then finishes with `finish` if the record's own remove plus the
/// map's `finish_cost` still fits, else flags the record as being cleared.
#[track_caller]
fn remove_step_in<M, V: Record + Removable>(
    map: &mut M,
    path: &Path,
    n: u32,
    finish_cost: impl FnOnce(&M) -> u32,
    finish: impl FnOnce(&mut M),
) -> Removal {
    if cache::get_entry::<V>(path).is_none() {
        return Removal::Done;
    }
    guard_removal::<V>(path, true);
    // Reserve the move to the flag key (1 remove of the record key on the first call that does
    // not finish), so a `Pending` call also stays within `n`.
    let pend = cache::clearing_cost(path);
    let mut budget = n - pend;
    if V::clear_children_some(path, &mut budget)
        && cache::delete_cost::<V>(path, true) + finish_cost(map) <= budget + pend
    {
        finish(map);
        Removal::Done
    } else {
        cache::set_clearing(path);
        Removal::Pending
    }
}

/// What one [`Map::remove_step`] / [`LookupMap::remove_step`] call left behind.
#[must_use = "`Pending` means the record is still there, being cleared: call `remove_step` again \
              in a later call until it returns `Done`"]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Removal {
    /// Entries remain. The record is flagged as being cleared and refuses writes to it or below
    /// it until a later `remove_step` finishes.
    Pending,
    /// The record is gone (or was absent).
    Done,
}

impl Removal {
    /// `true` for `Done`.
    pub fn is_done(self) -> bool {
        self == Removal::Done
    }
}

/// Iterator over a dense index range that reads only the positions it yields. `nth` (and so
/// `skip(n)`) advances the index without touching storage, so offset pagination is
/// `iter().skip(from).take(limit)` at the cost of the items returned.
pub struct IndexIter<F> {
    range: core::ops::Range<u32>,
    f: F,
}

impl<F> IndexIter<F> {
    fn new(len: u32, f: F) -> Self {
        IndexIter { range: 0..len, f }
    }
}

impl<T, F: FnMut(u32) -> T> Iterator for IndexIter<F> {
    type Item = T;
    fn next(&mut self) -> Option<T> {
        self.range.next().map(&mut self.f)
    }
    fn nth(&mut self, n: usize) -> Option<T> {
        self.range.nth(n).map(&mut self.f)
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.range.size_hint()
    }
    fn count(self) -> usize {
        self.range.len()
    }
}

impl<T, F: FnMut(u32) -> T> DoubleEndedIterator for IndexIter<F> {
    fn next_back(&mut self) -> Option<T> {
        self.range.next_back().map(&mut self.f)
    }
    fn nth_back(&mut self, n: usize) -> Option<T> {
        self.range.nth_back(n).map(&mut self.f)
    }
}

impl<T, F: FnMut(u32) -> T> ExactSizeIterator for IndexIter<F> {}

/// Marker for the read-only-view hint. Never implemented: calling a write method on an
/// [`At`] view fails with this message.
#[diagnostic::on_unimplemented(
    message = "views are read-only; use `at_mut` or `get_mut`",
    label = "`at(&k)` returns a read-only view",
    note = "`at` walks the path without reading; `at_mut(&k)` gives `&mut` handles to a record's collections, `get_mut(&k)` reads the whole record"
)]
pub trait ViewIsReadOnly {}

macro_rules! read_only {
    ($($name:ident ( $($arg:ty),* );)*) => {
        $(
            #[doc(hidden)]
            #[allow(unused_variables)]
            pub fn $name(&self, $(_: $arg),*) -> !
            where
                Self: ViewIsReadOnly,
            {
                unreachable!()
            }
        )*
    };
}

/// `Display`, `AsRef<str>` and `Error` for a unit error type, all from its `MSG`.
macro_rules! message_error {
    ($ty:ident) => {
        impl core::fmt::Display for $ty {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                f.write_str(Self::MSG)
            }
        }
        impl AsRef<str> for $ty {
            fn as_ref(&self) -> &str {
                Self::MSG
            }
        }
        impl std::error::Error for $ty {}
    };
}

/// The cursor key passed to `page` / `page_with` / `keys_page` was removed since the page that
/// returned it, so there is no position to continue from. Restart from a key you still hold,
/// from `None`, or switch to offset paging (`iter().skip(from).take(n)`).
///
/// Implements `AsRef<str>`, so a `#[handle_result]` view returning
/// `Result<Page<K, V>, CursorGone>` maps it to a panic with this message.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CursorGone;

impl CursorGone {
    const MSG: &'static str = "state: page cursor key no longer exists (removed since the last \
                               page); restart from a known key or use offset paging";
}
message_error!(CursorGone);

/// `try_insert(k, v)` found the key already present. Nothing was written and the stored value
/// was not read (only its existence was checked). Carries nothing, so `.expect("..")` works for
/// any value type, records without `Debug` included.
///
/// Implements `AsRef<str>`, so a `#[handle_result]` method can return it as is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Occupied;

impl Occupied {
    const MSG: &'static str = "state: key already present; nothing was inserted";
}
message_error!(Occupied);

/// One page of a cursor-paginated collection.
///
/// `revision` is the map's removal counter when the page was read (T4): two pages with equal
/// revisions had no removal between them, so together they are an exact walk. Different
/// revisions mean a removal happened and a key may have been skipped (the last key moves into
/// a removed key's slot); restart if that matters.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(crate = "crate::serde")]
pub struct Page<K, V> {
    pub items: std::vec::Vec<(K, V)>,
    pub next: Option<K>,
    pub revision: u64,
}

/// One page of keys. `revision` as on [`Page`].
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(crate = "crate::serde")]
pub struct KeyPage<K> {
    pub keys: std::vec::Vec<K>,
    pub next: Option<K>,
    pub revision: u64,
}

/// A collection handle reached through a view (or `GroupedMap::group`). Reads through it
/// live for `'a`, the borrow of the parent collection, not for the handle itself, so
/// `let bal = map.at(&id).balances.get(&t);` compiles.
pub struct At<'a, C> {
    inner: C,
    _m: PhantomData<&'a ()>,
}

impl<C> At<'_, C> {
    fn new(inner: C) -> Self {
        At { inner, _m: PhantomData }
    }
}

/// `Default`, `Debug`, `path` and `__bound_at` for a collection handle whose fields besides
/// `path` start as `{ field: init }` (default `_m: PhantomData`).
macro_rules! collection_value {
    ($ty:ident < $($g:ident),* >) => {
        collection_value!($ty<$($g),*> { _m: PhantomData });
    };
    ($ty:ident < $($g:ident),* > { $($f:ident: $init:expr),* }) => {
        impl<$($g),*> Default for $ty<$($g),*> {
            fn default() -> Self {
                Self { path: Binding::Unbound, $($f: $init),* }
            }
        }
        impl<$($g),*> core::fmt::Debug for $ty<$($g),*> {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                write!(f, "{}({:?})", stringify!($ty), self.path)
            }
        }
        impl<$($g),*> $ty<$($g),*> {
            /// The path this handle is bound to, if any.
            pub fn path(&self) -> Option<&Path> {
                match &self.path {
                    Binding::Bound(p, ..) => Some(p),
                    _ => None,
                }
            }
            #[doc(hidden)]
            pub fn __bound_at(path: Path) -> Self {
                Self { path: Binding::at(path), $($f: $init),* }
            }
        }
    };
}

/// `Value` for a collection whose `Binding` is `self.$binding` (`path`, or `0.path` for the
/// `Set` newtypes).
macro_rules! not_inline_value {
    ($($binding:tt).+) => {
        fn encode_value(&self, _: Option<&[u8]>) -> std::vec::Vec<u8> {
            unreachable!("collections are not stored inline")
        }
        fn decode_value(_: &[u8], _: &Path) -> Result<Self, RecordError> {
            not_inline()
        }
        fn bind(&mut self, path: &Path) {
            bind_slot(&mut self.$($binding).+, path)
        }
        fn detach(&mut self) {
            self.$($binding).+ = Binding::Detached;
        }
    };
}

/// `at` and `at_mut`, the same on `LookupMap` and `Map`.
macro_rules! record_views {
    () => {
        /// A read-only view bound to the path for `k`. Reads nothing by itself.
        #[track_caller]
        pub fn at(&self, k: &K) -> V::View<'_>
        where
            V: Record,
        {
            V::__view(self.entry_path(k))
        }

        /// A mutable view of the record at `k`: its collections and `Lazy` fields as `&mut`
        /// handles, its scalars unavailable. 1 `storage_has_key` (no bytes) unless cached; the
        /// record itself is not read. `None` if the record does not exist (std convention: named
        /// accessors return `Option`); an absent record costs a 2nd `storage_has_key`, of its D9
        /// flag key, and a record being cleared panics.
        #[track_caller]
        pub fn at_mut(&mut self, k: &K) -> Option<V::ViewMut<'_>>
        where
            V: Record,
        {
            self.path.live();
            record_at_mut::<V>(self.entry_path(k))
        }
    };
}

/// The reads of a map's `At` view, the same on `LookupMap` and `Map`.
macro_rules! at_map_reads {
    () => {
        #[track_caller]
        pub fn get(&self, k: &K) -> Option<&'a V> {
            cache::get_entry::<V>(&self.inner.entry_path(k)).map(|p| unsafe { r(p) })
        }
        #[track_caller]
        pub fn contains_key(&self, k: &K) -> bool {
            self.inner.contains_key(k)
        }
        #[track_caller]
        pub fn at(&self, k: &K) -> V::View<'a>
        where
            V: Record,
        {
            V::__view(self.inner.entry_path(k))
        }
    };
}

// ---------------------------------------------------------------------------------------
// LookupMap
// ---------------------------------------------------------------------------------------

/// Lookup-only map. Entries at `path ++ 00 ++ h16(k) ++ borsh(k)`.
pub struct LookupMap<K, V> {
    path: Binding,
    _m: PhantomData<fn() -> (K, V)>,
}
collection_value!(LookupMap<K, V>);

impl<K: StorageKey + 'static, V: Value> Value for LookupMap<K, V> {
    const INLINE: bool = false;
    const KIND: u8 = super::record::marker::LOOKUP_MAP;
    const __ROOT_ONLY: bool = true;
    const MAX_SUBPATH: usize = 1 + ENTRY_HASH_LEN + K::MAX_LEN + V::MAX_SUBPATH;
    not_inline_value!(path);
}

impl<K: StorageKey + 'static, V: Value> LookupMap<K, V> {
    const CHECK: () = assert!(V::INLINE, "a map value cannot itself be a collection; put it in a `#[near(storage)]` record");

    #[track_caller]
    fn entry_path(&self, k: &K) -> Path {
        let () = Self::CHECK;
        bound(&self.path).kind(kind::ENTRIES).entry(k)
    }

    /// 1 read.
    #[track_caller]
    pub fn get(&self, k: &K) -> Option<&V> {
        cache::get_entry::<V>(&self.entry_path(k)).map(|p| unsafe { r(p) })
    }

    /// 1 `storage_has_key` (no value bytes) unless the key is already cached.
    #[track_caller]
    pub fn contains_key(&self, k: &K) -> bool {
        cache::exists_entry::<V>(&self.entry_path(k))
    }

    /// 1 read; 1 write at flush if the value changed. Unknown tags are preserved.
    #[track_caller]
    pub fn get_mut(&mut self, k: &K) -> Option<&mut V> {
        self.path.live();
        unsafe { get_entry_mut(&self.entry_path(k)) }
    }

    /// Always overwrites: 1 write, no read, replaces the stored value whole (unknown tags
    /// included). Returns nothing, since knowing whether the key existed would cost a read.
    ///
    /// Only for values that own no collections (`V: Leaf`). For a record that owns collections
    /// use [`LookupMap::try_insert`] or `entry(k).or_default()`:
    ///
    /// ```compile_fail,E0277
    /// use near_sdk::near;
    /// use near_sdk::state::{LookupMap, Map};
    /// #[near(storage)]
    /// pub struct Market {
    ///     #[key(1)] orders: Map<u64, u128>,
    /// }
    /// fn place(markets: &mut LookupMap<u32, Market>) {
    ///     // error[E0277]: `Market` owns storage (a collection, a `Lazy`, or a record holding
    ///     // one), so `insert` cannot overwrite it
    ///     // note: use `try_insert(k, v)` (`Err(Occupied)` if the key exists, else the bound
    ///     // record) or `entry(k).or_default()`; to replace a record, `remove` it first
    ///     markets.insert(7, Market::default());
    /// }
    /// ```
    ///
    /// The fix:
    ///
    /// ```
    /// # use near_sdk::near;
    /// # use near_sdk::state::{LookupMap, Map};
    /// # #[near(storage)]
    /// # pub struct Market {
    /// #     #[key(1)] orders: Map<u64, u128>,
    /// # }
    /// fn place(markets: &mut LookupMap<u32, Market>) {
    ///     let m = markets.try_insert(7, Market::default()).expect("market exists");
    ///     m.orders.insert(1, 100);
    /// }
    /// ```
    #[track_caller]
    pub fn insert(&mut self, k: K, mut v: V)
    where
        V: Leaf,
    {
        self.path.live();
        let path = self.entry_path(&k);
        v.bind(&path);
        cache::put(&path, v);
    }

    /// Inserts only if the key is absent, and returns the value bound to its key; `Err(Occupied)`
    /// if present, with nothing written and the stored value not read. For every value type,
    /// leaf or record; the only way in for a record that owns collections besides `entry`.
    ///
    /// 1 `storage_has_key` + 1 write (a plain `insert` is 1 write, no check). For a value that
    /// owns collections a miss adds 1 `storage_has_key` on its D9 flag key, and a record being
    /// cleared by `remove_step` panics instead of returning `Occupied`.
    ///
    /// `require!(m.try_insert(k, v).is_ok(), "..")` or `m.try_insert(k, v).expect("..")` is the
    /// first-write-wins guard; plain `insert` would overwrite.
    #[track_caller]
    pub fn try_insert(&mut self, k: K, mut v: V) -> Result<&mut V, Occupied> {
        self.path.live();
        let path = self.entry_path(&k);
        if cache::exists_entry::<V>(&path) {
            assert_not_clearing::<V>(&path);
            return Err(Occupied);
        }
        v.bind(&path);
        Ok(unsafe { m(cache::put(&path, v)) })
    }

    /// 1 read, 1 write. The old value comes back detached, its collections cleared. Panics if
    /// the stored record carries a collection tag this code does not declare (see
    /// [`Removable`], API round 5).
    #[track_caller]
    pub fn replace(&mut self, k: K, mut v: V) -> Option<V>
    where
        V: Removable,
    {
        self.path.live();
        let path = self.entry_path(&k);
        let old = cache::get_entry::<V>(&path).map(|_| take_read::<V>(&path));
        v.bind(&path);
        cache::put(&path, v);
        old
    }

    /// 1 read, 1 remove, plus clearing the value's collections. Panics, changing nothing, if the
    /// stored record carries a collection tag this code does not declare (see [`Removable`]).
    /// The returned record is
    /// detached: its collections panic on use.
    #[track_caller]
    pub fn remove(&mut self, k: &K) -> Option<V>
    where
        V: Removable,
    {
        self.path.live();
        let path = self.entry_path(k);
        cache::get_entry::<V>(&path)?;
        Some(take_read(&path))
    }

    /// 1 remove, no read, plus clearing the value's collections. The remove is blind: it is
    /// issued even when the key is absent (unless this call already read it as absent), since
    /// nothing tells `discard` whether the key exists.
    ///
    /// A `#[near(storage)]` record value is read first (API round 5): 1 read, then 1 remove,
    /// none if absent. The read checks the stored record for a collection tag this code does
    /// not declare and panics if there is one (see [`Removable`]); it also says which key holds
    /// a record that owns collections, so that is 1 remove, not the blind 2 (record key and D9
    /// flag key) of earlier rounds.
    #[track_caller]
    pub fn discard(&mut self, k: &K)
    where
        V: Removable,
    {
        self.path.live();
        let path = self.entry_path(k);
        guard_removal::<V>(&path, true);
        V::clear_children(&path);
        cache::delete_entry::<V>(&path);
    }

    /// Removes the record at `k` in bounded steps (D9): each call issues at most `n`
    /// `storage_remove` host calls, then returns. `Removal::Pending` while anything remains
    /// (call again in a later call), `Removal::Done` once the record is gone (or was absent).
    ///
    /// While it is pending the record is stored at its clearing flag key ([`Path::flag`], its
    /// key plus `00`) and its own key is absent, so `get_mut`, `entry` and `at_mut` on it panic
    /// in every later call until clearing completes: nothing can be added below a record
    /// faster than it is cleared. `get` / `at` still read it.
    ///
    /// # The bound (API round 5, hazard A5)
    ///
    /// `n` counts `storage_remove` host calls of ANY kind: map entries, index and key-slot
    /// records, headers, `Lazy` values, `Vector` slots, nested records, the record itself and
    /// its flag key. One call issues at most `n` of them. Work is taken in whole units, each
    /// only if it fits in what is left: a map entry is 3 (entry, index, key slot), a `Lazy`
    /// value or a `Vector` slot 1, an emptied collection's header 1 (0 if it was never stored),
    /// the record itself 1 here (3 in a [`Map`]: record, index, key slot), and the move to the
    /// flag key on the first call that does not finish 1, reserved before clearing starts. So
    /// `n` must be at least [`MIN_REMOVE_STEP`] (3): `n` is a `NonZeroU32`, so 0 does not
    /// compile, and 1 or 2 panic. With `n >= 3` every call removes at least one key.
    ///
    /// On top of the `n` removes a call does bookkeeping that depends on the schema, not on how
    /// much the record holds. For a record whose collections hold plain values (one level of
    /// nesting, like `Account { balances: Map<..> }`), with `F` its `Map`/`Set`/`Vector` fields
    /// and `L` its `Lazy` fields, one call makes at most `n / 3 + F + 5` reads (the record, 2
    /// once flagged; each collection's header; one key slot per map entry removed, plus the
    /// next one when entries remain; finishing in a `Map`: the index, the header and the last
    /// slot), `L` `storage_has_key` (each `Lazy` is checked before its remove is charged, so a
    /// resumed call does not re-pay values already gone), and at most 3 writes (a partly
    /// cleared header and the flag key, or, finishing in a `Map`, the moved key's slot and
    /// index and the header). Each further level of nesting adds, per entry, 1 read of the
    /// nested record (the unknown-collection check of [`Removable`]) and its own header reads,
    /// and on the path where the call stops at most 1 lookahead read and 1 header write.
    ///
    /// `n = 0` does not compile:
    ///
    /// ```compile_fail,E0308
    /// # use near_sdk::near;
    /// # use near_sdk::state::{LookupMap, Map};
    /// # #[near(storage)]
    /// # pub struct Market {
    /// #     #[key(1)] orders: Map<u64, u128>,
    /// # }
    /// fn close(markets: &mut LookupMap<u32, Market>) {
    ///     let _ = markets.remove_step(&7, 0); // error[E0308]: expected `NonZero<u32>`
    /// }
    /// ```
    ///
    /// A budget is a constant:
    ///
    /// ```
    /// # use near_sdk::near;
    /// # use near_sdk::state::{LookupMap, Map, Removal};
    /// # #[near(storage)]
    /// # pub struct Market {
    /// #     #[key(1)] orders: Map<u64, u128>,
    /// # }
    /// use core::num::NonZeroU32;
    /// /// About 100 orders per call (3 removes each).
    /// const STEP: NonZeroU32 = NonZeroU32::new(300).unwrap();
    /// fn close(markets: &mut LookupMap<u32, Market>) -> bool {
    ///     markets.remove_step(&7, STEP) == Removal::Pending
    /// }
    /// ```
    #[track_caller]
    pub fn remove_step(&mut self, k: &K, n: NonZeroU32) -> Removal
    where
        V: Record + Removable,
    {
        self.path.live();
        let n = check_step(n);
        let path = self.entry_path(k);
        remove_step_in::<_, V>(self, &path, n, |_| 0, |s| s.discard(k))
    }

    #[track_caller]
    pub fn entry(&mut self, k: K) -> LookupMapEntry<'_, K, V> {
        self.path.live();
        let path = self.entry_path(&k);
        LookupMapEntry { path, _m: PhantomData }
    }

    record_views!();
}

impl<'a, K: StorageKey + 'static, V: Value> At<'a, LookupMap<K, V>> {
    read_only! {
        insert(K, V);
        try_insert(K, V);
        remove_step(&K, NonZeroU32);
        remove(&K);
        discard(&K);
        get_mut(&K);
        entry(K);
        at_mut(&K);
    }
    at_map_reads!();
}

pub struct LookupMapEntry<'a, K, V> {
    path: Path,
    _m: PhantomData<(&'a mut LookupMap<K, V>, K)>,
}

impl<'a, K, V: Value> LookupMapEntry<'a, K, V> {
    /// 1 read; if absent, 1 write at flush.
    #[track_caller]
    pub fn or_insert_with(self, f: impl FnOnce() -> V) -> &'a mut V {
        if let Some(v) = unsafe { get_entry_mut(&self.path) } {
            return v;
        }
        let mut v = f();
        v.bind(&self.path);
        unsafe { m(cache::put(&self.path, v)) }
    }
    #[track_caller]
    pub fn or_insert(self, v: V) -> &'a mut V {
        self.or_insert_with(|| v)
    }
    #[track_caller]
    pub fn or_default(self) -> &'a mut V
    where
        V: Default,
    {
        self.or_insert_with(V::default)
    }
}

// ---------------------------------------------------------------------------------------
// Map
// ---------------------------------------------------------------------------------------

/// `LookupMap` plus a dense key vector: entries `00`, index `01`, key slots `02`, header `03`.
pub struct Map<K, V> {
    path: Binding,
    _m: PhantomData<fn() -> (K, V)>,
}
collection_value!(Map<K, V>);

impl<K: StorageKey + Clone + 'static, V: Value> Value for Map<K, V> {
    const INLINE: bool = false;
    const KIND: u8 = super::record::marker::MAP;
    const MAX_SUBPATH: usize = 1 + ENTRY_HASH_LEN + K::MAX_LEN + V::MAX_SUBPATH;
    not_inline_value!(path);
}

impl<K: StorageKey + Clone + 'static, V: Removable> Removable for Map<K, V> {
    fn clear_children(path: &Path) {
        Map::<K, V>::__bound_at(path.clone()).clear_owned();
    }
    fn clear_children_some(path: &Path, budget: &mut u32) -> bool {
        Map::<K, V>::__bound_at(path.clone()).clear_budgeted(budget)
    }
}

impl<K: StorageKey + Clone + 'static, V: Value> Map<K, V> {
    const CHECK: () = assert!(V::INLINE, "a map value cannot itself be a collection; put it in a `#[near(storage)]` record");

    #[track_caller]
    fn p(&self) -> &Path {
        let () = Self::CHECK;
        bound(&self.path)
    }
    fn entry_path(&self, k: &K) -> Path {
        self.p().kind(kind::ENTRIES).entry(k)
    }
    fn index_path(&self, k: &K) -> Path {
        self.p().kind(kind::INDEX).entry(k)
    }
    fn slot_path(&self, i: u32) -> Path {
        self.p().kind(kind::SLOTS).slot(i)
    }
    fn header(&self) -> Path {
        self.p().kind(kind::HEADER)
    }
    fn bound_anchored(path: Path, anchor: usize, kind: u8) -> Self {
        Map { path: Binding::anchored(path, anchor, kind), _m: PhantomData }
    }

    fn head(&self) -> MapHeader {
        cache::get::<MapHeader>(&self.header()).map(|p| unsafe { *r(p) }).unwrap_or_default()
    }

    /// One removal (or one batch of removals) that leaves `len` entries: bumps the revision.
    fn set_len_after_removal(&self, len: u32) {
        let h = self.head();
        put_header(&self.header(), MapHeader { len, revision: h.revision.wrapping_add(1) });
    }

    /// 1 read.
    pub fn len(&self) -> u32 {
        self.head().len
    }
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// The map's revision: a counter in its header that every removal bumps (T4). 1 read, the
    /// header, which `len` and every page read anyway. Starts at 0 and never goes back, even
    /// when the map empties; only removing the record that owns the map resets it.
    pub fn revision(&self) -> u64 {
        self.head().revision
    }

    /// 1 read.
    #[track_caller]
    pub fn get(&self, k: &K) -> Option<&V> {
        cache::get_entry::<V>(&self.entry_path(k)).map(|p| unsafe { r(p) })
    }

    /// 1 `storage_has_key` unless cached.
    #[track_caller]
    pub fn contains_key(&self, k: &K) -> bool {
        cache::exists_entry::<V>(&self.entry_path(k))
    }

    /// 1 read; 1 write at flush if the value changed.
    #[track_caller]
    pub fn get_mut(&mut self, k: &K) -> Option<&mut V> {
        self.path.live();
        unsafe { get_entry_mut(&self.entry_path(k)) }
    }

    /// Appends `k` to the key vector. A header absent before this write (the map has never held
    /// an entry, or its record was just created) is the A3 first-write signal (round 7): the
    /// record storing this map must declare it, see `Binding::declare`.
    #[track_caller]
    fn push_key(&mut self, k: &K) {
        let h = self.head();
        if h == MapHeader::default() {
            self.path.declare(&|| true);
        }
        cache::put(&self.index_path(k), h.len);
        cache::put(&self.slot_path(h.len), k.clone());
        put_header(&self.header(), MapHeader { len: h.len.checked_add(1).expect("Map is full"), ..h });
    }

    /// Always overwrites. Returns `true` if the key was new (free: the index read says so).
    ///
    /// New key: 1 index read, 1 header read, 4 writes. Existing key: 1 index read, 1 write.
    /// First-write-wins is [`Map::try_insert`] (same cost) or `entry(k).or_insert(v)`; do not
    /// write `require!(m.insert(k, v), "..")`, which has already overwritten when it fails.
    ///
    /// Only for values that own no collections (`V: Leaf`); see [`LookupMap::insert`] for the
    /// rejected call and the fix. For a record that owns collections use [`Map::try_insert`] or
    /// `entry(k).or_default()`.
    ///
    /// Not `#[must_use]`, like `HashMap::insert` and `HashSet::insert`: discarding the bool is the
    /// common overwrite case.
    #[track_caller]
    pub fn insert(&mut self, k: K, mut v: V) -> bool
    where
        V: Leaf,
    {
        self.path.live();
        let path = self.entry_path(&k);
        let exists = cache::get::<u32>(&self.index_path(&k)).is_some();
        v.bind(&path);
        cache::put(&path, v);
        if !exists {
            self.push_key(&k);
        }
        !exists
    }

    /// Inserts only if the key is absent, and returns the value bound to its key; `Err(Occupied)`
    /// if present, with nothing written and the stored value not read. For every value type,
    /// leaf or record; the only way in for a record that owns collections besides `entry`.
    ///
    /// Same cost as `insert`: 1 index read, then for a new key 1 header read and 4 writes; an
    /// existing key costs the index read only. A record being cleared by `remove_step` still
    /// has its index, so it is `Occupied`.
    #[track_caller]
    pub fn try_insert(&mut self, k: K, mut v: V) -> Result<&mut V, Occupied> {
        self.path.live();
        let path = self.entry_path(&k);
        if cache::get::<u32>(&self.index_path(&k)).is_some() {
            return Err(Occupied);
        }
        v.bind(&path);
        let p = cache::put(&path, v);
        self.push_key(&k);
        Ok(unsafe { m(p) })
    }

    #[track_caller]
    pub fn entry(&mut self, k: K) -> MapEntry<'_, K, V> {
        self.path.live();
        MapEntry { map: self, k }
    }

    fn remove_inner(&mut self, k: &K, read_value: bool) -> Option<Option<V>>
    where
        V: Removable,
    {
        let path = self.entry_path(k);
        let idx_path = self.index_path(k);
        let mut value = None;
        if read_value {
            cache::get_entry::<V>(&path)?;
        }
        let i = unsafe { *r(cache::get::<u32>(&idx_path)?) };
        guard_removal::<V>(&path, true);
        let len = self.len();
        V::clear_children(&path);
        if read_value || core::mem::size_of::<V>() == 0 {
            if let Some(mut v) = cache::take_entry::<V>(&path) {
                v.detach();
                value = Some(v);
            }
        } else {
            // `discard`: nothing is handed back, so a cached value is buried, not freed.
            cache::delete_entry::<V>(&path);
        }
        cache::delete::<u32>(&idx_path);
        let last = len - 1;
        let last_slot = self.slot_path(last);
        if i != last {
            let moved: K = unsafe { r(cache::get::<K>(&last_slot).expect("dense key vector")) }.clone();
            cache::put(&self.slot_path(i), moved.clone());
            cache::put(&self.index_path(&moved), i);
        }
        cache::delete::<K>(&last_slot);
        self.set_len_after_removal(last);
        Some(value)
    }

    /// 1 entry read, 1 index read, 1 header read, 2 removes (entry, index), 1 read and 1
    /// remove of the last slot, 2 writes for the moved key, 1 header write (also when the map
    /// empties: the header keeps the revision, T4). If the key is last: no moved key, so neither
    /// the last-slot read nor the 2 moved-key writes (3 reads in all; 2 for `discard`).
    ///
    /// A zero-sized `V` (such as `()`) skips the entry read: the index read already says
    /// whether the key exists, and the value carries no bytes. So `Map<K, ()>` and
    /// `GroupedMap<G, K, ()>` remove at the cost of `discard`.
    #[track_caller]
    pub fn remove(&mut self, k: &K) -> Option<V>
    where
        V: Removable,
    {
        self.path.live();
        if core::mem::size_of::<V>() == 0 {
            let path = self.entry_path(k);
            return match self.remove_inner(k, false)? {
                Some(v) => Some(v),
                // Not cached: rebuild the (byte-less) value instead of reading it.
                None => Some(V::decode_value(&[], &path).unwrap_or_else(|e| {
                    panic!("state: zero-sized value does not decode from no bytes: {e}")
                })),
            };
        }
        self.remove_inner(k, true).flatten()
    }

    /// As `remove` without the entry read. For a `#[near(storage)]` record value the entry is
    /// read anyway (the unknown-collection check, API round 5; see [`Removable`]), so `discard`
    /// then differs from `remove` only by the decode, and a record that owns collections is 1
    /// remove instead of the blind 2.
    #[track_caller]
    pub fn discard(&mut self, k: &K) -> bool
    where
        V: Removable,
    {
        self.path.live();
        self.remove_inner(k, false).is_some()
    }

    /// The `i`th key in key-vector order: 1 read.
    pub fn key_at(&self, i: u32) -> Option<K> {
        cache::get::<K>(&self.slot_path(i)).map(|p| unsafe { r(p) }.clone())
    }

    fn cursor_start(&self, after: Option<&K>) -> Result<u32, CursorGone> {
        match after {
            None => Ok(0),
            Some(k) => {
                let i = cache::get::<u32>(&self.index_path(k)).ok_or(CursorGone)?;
                Ok((unsafe { *r(i) }) + 1)
            }
        }
    }

    /// Cursor page: with a cursor, 2 reads (cursor index, header) plus 2 per item.
    /// Without a cursor, 1 read plus 2 per item. `Err(CursorGone)` if the cursor key was
    /// removed since the page that returned it (1 read, the cursor's index).
    ///
    /// Cursor and offset paging both assume the map does not change between pages. A cursor
    /// whose key was removed fails loudly. Otherwise a removal between pages can SKIP a key,
    /// with either kind of paging: it moves the last key into the hole, ahead of where the next
    /// page starts. Cursor paging can also return keys TWICE: removals can move the cursor key
    /// itself toward the front (each moves the then-last key into its hole, and the cursor key
    /// can become the last key), and the next page then restarts from that earlier position
    /// and repeats keys already returned (offset paging never repeats). Every
    /// page carries the map's `revision` (T4, free: from the header read): a caller that needs
    /// an exact walk compares it across pages, and equal revisions mean no removal happened in
    /// between.
    ///
    /// `limit` 0 returns no items and `next: None` even when entries remain, so `next == None`
    /// means the end only for `limit > 0`.
    ///
    /// Needs `V: Clone`, which a record holding a collection never is: use [`Map::page_with`]
    /// and map each record to the type you return.
    pub fn page(&self, after: Option<&K>, limit: u32) -> Result<Page<K, V>, CursorGone>
    where
        V: Clone,
    {
        self.page_with(after, limit, |_, v| v.clone())
    }

    /// Cursor page with each value mapped by `f`: same reads and errors as `page`. The path
    /// for record-valued maps: `map.page_with(after, 50, |id, p| ProposalJson::new(id, p))`.
    pub fn page_with<T>(
        &self,
        after: Option<&K>,
        limit: u32,
        mut f: impl FnMut(&K, &V) -> T,
    ) -> Result<Page<K, T>, CursorGone> {
        let start = self.cursor_start(after)?;
        let MapHeader { len, revision } = self.head();
        let end = start.saturating_add(limit).min(len);
        let mut items = std::vec::Vec::with_capacity((end.saturating_sub(start)) as usize);
        for i in start..end {
            let k = self.key_at(i).expect("dense key vector");
            let v = f(&k, self.get(&k).expect("indexed key has an entry"));
            items.push((k, v));
        }
        let next = if end < len { items.last().map(|(k, _)| k.clone()) } else { None };
        Ok(Page { items, next, revision })
    }

    /// Key-only cursor page: as `page`, 1 read per key; same `CursorGone` error.
    pub fn keys_page(&self, after: Option<&K>, limit: u32) -> Result<KeyPage<K>, CursorGone> {
        let start = self.cursor_start(after)?;
        let MapHeader { len, revision } = self.head();
        let end = start.saturating_add(limit).min(len);
        let keys: std::vec::Vec<K> =
            (start..end).map(|i| self.key_at(i).expect("dense key vector")).collect();
        let next = if end < len { keys.last().cloned() } else { None };
        Ok(KeyPage { keys, next, revision })
    }

    /// 2 reads per entry yielded, plus 1 for the length. Order is the key vector's: insertion
    /// order until a remove moves the last key into the hole. `skip(n)` is free, so offset
    /// pagination is `iter().skip(from).take(limit)`.
    pub fn iter<'s>(&'s self) -> IndexIter<impl FnMut(u32) -> (K, &'s V) + 's> {
        IndexIter::new(self.len(), move |i| {
            let k = self.key_at(i).expect("dense key vector");
            let v = cache::get_entry::<V>(&self.entry_path(&k)).expect("indexed key has an entry");
            (k, unsafe { r(v) })
        })
    }

    /// 1 read per key yielded, plus 1 for the length. `skip(n)` is free.
    pub fn keys(&self) -> IndexIter<impl FnMut(u32) -> K + '_> {
        IndexIter::new(self.len(), move |i| self.key_at(i).expect("dense key vector"))
    }

    /// Removes everything: 1 header read, then per entry 1 slot read and 3 removes (entry,
    /// index, slot), plus clearing each value's own collections; then 1 header write that keeps
    /// the revision (T4; nothing if the map was already empty). A `#[near(storage)]` record
    /// value adds 1 read per entry (the unknown-collection check, API round 5; panics if one
    /// carries an undeclared collection tag), and its entry is then 1 remove even when it owns
    /// collections (was 2 blind: record key and flag key).
    pub fn clear(&mut self)
    where
        V: Removable,
    {
        self.path.live();
        self.clear_some(u32::MAX);
    }

    /// Removes at most `n` entries from the end. Returns `true` if entries remain. Bumps the
    /// revision once per call that removes anything.
    pub fn clear_some(&mut self, n: u32) -> bool
    where
        V: Removable,
    {
        self.path.live();
        let len = self.len();
        let stop = len.saturating_sub(n);
        if stop == len {
            return false;
        }
        for i in (stop..len).rev() {
            let slot = self.slot_path(i);
            let k: K = unsafe { r(cache::get::<K>(&slot).expect("dense key vector")) }.clone();
            let path = self.entry_path(&k);
            guard_removal::<V>(&path, true);
            V::clear_children(&path);
            cache::delete_entry::<V>(&path);
            cache::delete::<u32>(&self.index_path(&k));
            cache::delete::<K>(&slot);
        }
        self.set_len_after_removal(stop);
        stop > 0
    }

    /// `clear` for a map whose owner is being removed: also removes the header, revision and
    /// all, so a removed record leaves no key behind.
    fn clear_owned(&mut self)
    where
        V: Removable,
    {
        self.clear();
        cache::delete::<MapHeader>(&self.header());
    }

    /// Bounded clear from the end (D9), clearing each value's own collections first; see
    /// [`Removable::clear_children_some`]. Since API round 5 `budget` is host removes: an entry
    /// is its entry, index and key-slot removes (3), taken whole or not at all, and the header
    /// (1, when it exists) goes once the map is empty.
    fn clear_budgeted(&mut self, budget: &mut u32) -> bool
    where
        V: Removable,
    {
        loop {
            let len = self.len();
            if len == 0 {
                // The owner is being removed: the header goes too (T4).
                return spend(budget, cache::delete_cost::<MapHeader>(&self.header(), false), || {
                    cache::delete::<MapHeader>(&self.header())
                });
            }
            let i = len - 1;
            let slot = self.slot_path(i);
            let k: K = unsafe { r(cache::get::<K>(&slot).expect("dense key vector")) }.clone();
            let path = self.entry_path(&k);
            let idx = self.index_path(&k);
            guard_removal::<V>(&path, true);
            if !V::clear_children_some(&path, budget) {
                return false;
            }
            let cost = cache::delete_cost::<V>(&path, true)
                + cache::delete_cost::<u32>(&idx, false)
                + cache::delete_cost::<K>(&slot, false);
            if !spend(budget, cost, || {
                cache::delete_entry::<V>(&path);
                cache::delete::<u32>(&idx);
                cache::delete::<K>(&slot);
            }) {
                return false;
            }
            self.set_len_after_removal(i);
        }
    }

    /// Removes the record at `k` in bounded steps (D9): as [`LookupMap::remove_step`], at most
    /// `n` `storage_remove` host calls per call (`n >= 3`), and the key leaves the key vector in
    /// the call that finishes, whose 3 removes (record, index, last key slot) must fit in what
    /// the call has left. `Removal::Pending` while anything remains, `Removal::Done` once the
    /// record is gone (or was absent).
    #[track_caller]
    pub fn remove_step(&mut self, k: &K, n: NonZeroU32) -> Removal
    where
        V: Record + Removable,
    {
        self.path.live();
        let n = check_step(n);
        let path = self.entry_path(k);
        // Finishing also removes the index and the last key slot (counted as 1 without reading
        // the header to find it: every live map has its last slot stored).
        let finish = |s: &Self| cache::delete_cost::<u32>(&s.index_path(k), false) + 1;
        remove_step_in::<_, V>(self, &path, n, finish, |s| {
            s.remove_inner(k, false);
        })
    }

    record_views!();
}

impl<'a, K: StorageKey + Clone + 'static, V: Value> At<'a, Map<K, V>> {
    read_only! {
        insert(K, V);
        try_insert(K, V);
        remove_step(&K, NonZeroU32);
        remove(&K);
        discard(&K);
        get_mut(&K);
        entry(K);
        at_mut(&K);
        clear();
    }
    at_map_reads!();
    pub fn len(&self) -> u32 {
        self.inner.len()
    }
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }
    pub fn key_at(&self, i: u32) -> Option<K> {
        self.inner.key_at(i)
    }
    pub fn revision(&self) -> u64 {
        self.inner.revision()
    }
    pub fn page(&self, after: Option<&K>, limit: u32) -> Result<Page<K, V>, CursorGone>
    where
        V: Clone,
    {
        self.inner.page(after, limit)
    }
    pub fn page_with<T>(
        &self,
        after: Option<&K>,
        limit: u32,
        mut f: impl FnMut(&K, &'a V) -> T,
    ) -> Result<Page<K, T>, CursorGone> {
        // The reads live for 'a (see `get`), so `f` may keep them.
        self.inner.page_with(after, limit, |k, v| f(k, unsafe { &*(v as *const V) }))
    }
    pub fn keys_page(&self, after: Option<&K>, limit: u32) -> Result<KeyPage<K>, CursorGone> {
        self.inner.keys_page(after, limit)
    }
    /// As `Map::iter`; `skip(n)` is free.
    pub fn iter(&self) -> IndexIter<impl FnMut(u32) -> (K, &'a V) + use<'a, K, V>> {
        let map = Map::<K, V>::__bound_at(self.inner.p().clone());
        IndexIter::new(map.len(), move |i| {
            let k = map.key_at(i).expect("dense key vector");
            let v = cache::get_entry::<V>(&map.entry_path(&k)).expect("indexed key has an entry");
            (k, unsafe { r(v) })
        })
    }
    /// As `Map::keys`; `skip(n)` is free.
    pub fn keys(&self) -> IndexIter<impl FnMut(u32) -> K + use<'a, K, V>> {
        let map = Map::<K, V>::__bound_at(self.inner.p().clone());
        IndexIter::new(map.len(), move |i| map.key_at(i).expect("dense key vector"))
    }
}

pub struct MapEntry<'a, K, V> {
    map: &'a mut Map<K, V>,
    k: K,
}

impl<'a, K: StorageKey + Clone + 'static, V: Value> MapEntry<'a, K, V> {
    /// Existing: 1 read. New: 1 entry read, 1 header read, 4 writes (no index read: an absent
    /// entry implies an absent index).
    #[track_caller]
    pub fn or_insert_with(self, f: impl FnOnce() -> V) -> &'a mut V {
        let path = self.map.entry_path(&self.k);
        if let Some(v) = unsafe { get_entry_mut(&path) } {
            return v;
        }
        let mut v = f();
        v.bind(&path);
        let p = cache::put(&path, v);
        self.map.push_key(&self.k);
        unsafe { m(p) }
    }
    #[track_caller]
    pub fn or_insert(self, v: V) -> &'a mut V {
        self.or_insert_with(|| v)
    }
    #[track_caller]
    pub fn or_default(self) -> &'a mut V
    where
        V: Default,
    {
        self.or_insert_with(V::default)
    }
}

// ---------------------------------------------------------------------------------------
// Vector
// ---------------------------------------------------------------------------------------

/// Dense slots `00 ++ u32` and a length header `03`.
pub struct Vector<T> {
    path: Binding,
    _m: PhantomData<fn() -> T>,
}
collection_value!(Vector<T>);

impl<T: Value> Value for Vector<T> {
    const INLINE: bool = false;
    const KIND: u8 = super::record::marker::VECTOR;
    const MAX_SUBPATH: usize = 1 + 4 + T::MAX_SUBPATH;
    not_inline_value!(path);
}

impl<T: Removable> Removable for Vector<T> {
    fn clear_children(path: &Path) {
        Vector::<T>::__bound_at(path.clone()).clear();
    }
    /// Budget in host removes (API round 5): a slot is 1, and the last slot also takes the
    /// header with it (2).
    fn clear_children_some(path: &Path, budget: &mut u32) -> bool {
        let v = Vector::<T>::__bound_at(path.clone());
        loop {
            let len = v.len();
            if len == 0 {
                return true;
            }
            let slot = v.slot(len - 1);
            guard_removal::<T>(&slot, false);
            if !T::clear_children_some(&slot, budget) {
                return false;
            }
            let header = if len == 1 { cache::delete_cost::<u32>(&v.header(), false) } else { 0 };
            if !spend(budget, cache::delete_cost::<T>(&slot, false) + header, || {
                cache::delete::<T>(&slot);
                put_header(&v.header(), len - 1);
            }) {
                return false;
            }
        }
    }
}

impl<T: Value> Vector<T> {
    const CHECK: () = assert!(T::INLINE, "a Vector element cannot itself be a collection");
    /// Elements that own a subtree cannot move between slots: their collections would stay
    /// behind at the old slot's path.
    const CHECK_MOVABLE: () =
        assert!(T::MAX_SUBPATH == 0, "swap_remove would move a record away from its collections");

    #[track_caller]
    fn p(&self) -> &Path {
        let () = Self::CHECK;
        bound(&self.path)
    }
    fn slot(&self, i: u32) -> Path {
        self.p().kind(kind::ENTRIES).slot(i)
    }
    fn header(&self) -> Path {
        self.p().kind(kind::HEADER)
    }
    fn bound_anchored(path: Path, anchor: usize) -> Self {
        Vector { path: Binding::anchored(path, anchor, super::record::marker::VECTOR), _m: PhantomData }
    }

    /// 1 read.
    pub fn len(&self) -> u32 {
        cache::get::<u32>(&self.header()).map(|p| unsafe { *r(p) }).unwrap_or(0)
    }
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// 1 read (bounds are not checked against the header; a missing slot is `None`).
    pub fn get(&self, i: u32) -> Option<&T> {
        cache::get::<T>(&self.slot(i)).map(|p| unsafe { r(p) })
    }

    pub fn get_mut(&mut self, i: u32) -> Option<&mut T> {
        self.path.live();
        let path = self.slot(i);
        let p = cache::get::<T>(&path)?;
        cache::touch(&path);
        Some(unsafe { m(p) })
    }

    /// 1 header read, 1 write, 1 header write.
    ///
    /// A push into an empty vector (no header) below a record reached through `at_mut` is the A3
    /// first-write case (round 7): 1 read of the record, and 1 write of it if its kind marker
    /// for this field is missing.
    #[track_caller]
    pub fn push(&mut self, mut v: T) {
        self.path.live();
        let len = self.len();
        if len == 0 {
            self.path.declare(&|| true);
        }
        let path = self.slot(len);
        v.bind(&path);
        cache::put(&path, v);
        put_header(&self.header(), len + 1);
    }

    /// 1 header read, 1 slot read (the returned value), 1 remove, 1 header write.
    pub fn pop(&mut self) -> Option<T>
    where
        T: Removable,
    {
        self.path.live();
        let len = self.len();
        if len == 0 {
            return None;
        }
        let path = self.slot(len - 1);
        cache::get::<T>(&path);
        guard_removal::<T>(&path, false);
        T::clear_children(&path);
        let mut v = cache::take::<T>(&path);
        if let Some(v) = v.as_mut() {
            v.detach();
        }
        put_header(&self.header(), len - 1);
        v
    }

    /// 1 header read, 2 reads (slot `i`, last), 1 write, 1 remove, 1 header write. The write of
    /// slot `i` is skipped when the moved last element's bytes equal the removed one's (slot `i`
    /// was read, so the cache elides an identical rewrite).
    #[track_caller]
    pub fn swap_remove(&mut self, i: u32) -> T {
        self.path.live();
        let () = Self::CHECK_MOVABLE;
        let len = self.len();
        assert!(i < len, "state::Vector index {i} out of bounds (len {len})");
        let path = self.slot(i);
        cache::get::<T>(&path).expect("dense");
        guard_removal::<T>(&path, false);
        let v = cache::take::<T>(&path).expect("just read");
        let last = len - 1;
        if i != last {
            let last_path = self.slot(last);
            cache::get::<T>(&last_path).expect("dense");
            let moved = cache::take::<T>(&last_path).expect("just read");
            cache::put(&path, moved);
        }
        put_header(&self.header(), last);
        v
    }

    /// 1 header read, then 1 remove per element (no element reads; a `#[near(storage)]` record
    /// element is read, the unknown-collection check of API round 5), plus clearing each
    /// element's own collections; 1 header remove.
    pub fn clear(&mut self)
    where
        T: Removable,
    {
        self.path.live();
        let len = self.len();
        for i in 0..len {
            let path = self.slot(i);
            guard_removal::<T>(&path, false);
            T::clear_children(&path);
            cache::delete::<T>(&path);
        }
        put_header(&self.header(), 0);
    }

    /// 1 read per element yielded, plus 1 for the length. `skip(n)` is free.
    pub fn iter<'s>(&'s self) -> IndexIter<impl FnMut(u32) -> &'s T + 's> {
        IndexIter::new(self.len(), move |i| self.get(i).expect("dense"))
    }
}

impl<'a, T: Value> At<'a, Vector<T>> {
    read_only! {
        push(T);
        pop();
        get_mut(u32);
        swap_remove(u32);
        clear();
    }
    /// As `Vector::iter`; `skip(n)` is free.
    pub fn iter(&self) -> IndexIter<impl FnMut(u32) -> &'a T + use<'a, T>> {
        let v = Vector::<T>::__bound_at(self.inner.p().clone());
        IndexIter::new(v.len(), move |i| {
            cache::get::<T>(&v.slot(i)).map(|p| unsafe { r(p) }).expect("dense")
        })
    }
    pub fn get(&self, i: u32) -> Option<&'a T> {
        cache::get::<T>(&self.inner.slot(i)).map(|p| unsafe { r(p) })
    }
    pub fn len(&self) -> u32 {
        self.inner.len()
    }
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }
}

// ---------------------------------------------------------------------------------------
// Lazy
// ---------------------------------------------------------------------------------------

/// One value at its own key (`parent ++ tag`), read only when touched.
///
/// `Lazy::new(v)` holds `v` until the record is placed (by `insert`, `or_insert`, or `bind` on
/// the root) and writes it then, so a struct literal can carry the value:
/// `tokens.insert(id, Token { owner, metadata: Lazy::new(m), ..Default::default() })`. A
/// `Lazy::new` that is never placed writes nothing. `Default` is an empty `Lazy`.
///
/// The value cannot itself be a collection or `Lazy` (it has no inline encoding); put it in a
/// `#[near(storage)]` record:
///
/// ```compile_fail,E0080
/// use near_sdk::state::{Lazy, Set};
/// fn main() {
///     let l: Lazy<Set<u8>> = Lazy::default();
///     let _ = l.get(); // error[E0080]: a Lazy value cannot itself be a collection
/// }
/// ```
pub struct Lazy<T> {
    path: Binding,
    pending: core::cell::Cell<Option<T>>,
}

collection_value!(Lazy<T> { pending: core::cell::Cell::new(None) });

impl<T> Lazy<T> {
    /// A `Lazy` holding `v`, written when its record is placed. See the type docs.
    pub fn new(v: T) -> Self {
        Lazy { path: Binding::Unbound, pending: core::cell::Cell::new(Some(v)) }
    }
    fn bound_anchored(path: Path, anchor: usize) -> Self {
        Lazy {
            path: Binding::anchored(path, anchor, super::record::marker::LAZY),
            pending: core::cell::Cell::new(None),
        }
    }
}

impl<T: Value> Value for Lazy<T> {
    const INLINE: bool = false;
    const KIND: u8 = super::record::marker::LAZY;
    const MAX_SUBPATH: usize = T::MAX_SUBPATH;
    fn encode_value(&self, _: Option<&[u8]>) -> std::vec::Vec<u8> {
        unreachable!("collections are not stored inline")
    }
    fn decode_value(_: &[u8], _: &Path) -> Result<Self, RecordError> {
        not_inline()
    }
    /// Binds, then writes a pending `Lazy::new` value (1 write; a record value that owns
    /// collections is insert-new, as `set`).
    #[track_caller]
    fn bind(&mut self, path: &Path) {
        bind_slot(&mut self.path, path);
        if let Some(v) = self.pending.take() {
            self.set(v);
        }
    }
    fn detach(&mut self) {
        self.path = Binding::Detached;
        self.pending.set(None);
    }
    #[track_caller]
    fn __write_pending(&self, path: &Path) {
        let () = Lazy::<T>::CHECK;
        if let Some(mut v) = self.pending.take() {
            assert_insert_new::<T>(path);
            v.bind(path);
            cache::put(path, v);
        }
    }
}

impl<T: Removable> Removable for Lazy<T> {
    fn clear_children(path: &Path) {
        guard_removal::<T>(path, false);
        T::clear_children(path);
        cache::delete::<T>(path);
    }
    /// The value's own collections, then the value: its remove counts against the budget like
    /// any other (API round 5, hazard A5; before, a `Lazy` value was removed outside the budget).
    /// Unlike the unbounded clear, the remove is not blind: 1 `storage_has_key` first (answered
    /// from the cache when this call already read the value), and an absent value costs no
    /// remove. Without the check a resumed clear would re-pay a blind remove for every `Lazy` an
    /// earlier call already removed, and could never get past them.
    fn clear_children_some(path: &Path, budget: &mut u32) -> bool {
        guard_removal::<T>(path, false);
        if !T::clear_children_some(path, budget) {
            return false;
        }
        if !cache::exists(path) {
            return true;
        }
        spend(budget, cache::delete_cost::<T>(path, false), || cache::delete::<T>(path))
    }
}

impl<T: Value> Lazy<T> {
    const CHECK: () = assert!(T::INLINE, "a Lazy value cannot itself be a collection; put it in a `#[near(storage)]` record");

    #[track_caller]
    fn p(&self) -> &Path {
        let () = Self::CHECK;
        bound(&self.path)
    }

    /// 1 read.
    #[track_caller]
    pub fn get(&self) -> Option<&T> {
        cache::get::<T>(self.p()).map(|p| unsafe { r(p) })
    }
    #[track_caller]
    pub fn get_mut(&mut self) -> Option<&mut T> {
        self.path.live();
        let path = self.p().clone();
        let p = cache::get::<T>(&path)?;
        cache::touch(&path);
        Some(unsafe { m(p) })
    }
    /// 1 write (a record that owns collections: panics if one is already stored, since
    /// overwriting it would orphan its collections).
    ///
    /// Through `at_mut`, when this call has not read the record holding the `Lazy`, 1
    /// `storage_has_key` of the value first (round 7, the A3 first-write rule; free for a record
    /// value, whose insert-new check already made it): if the value is absent, 1 read of the
    /// record, and 1 write of it if its kind marker for this field is missing.
    #[track_caller]
    pub fn set(&mut self, mut v: T) {
        self.path.live();
        let path = self.p().clone();
        assert_insert_new::<T>(&path);
        self.path.declare(&|| !cache::exists(&path));
        v.bind(&path);
        cache::put(&path, v);
    }
    /// 1 remove, no read. The remove is blind: it is issued even when the `Lazy` is empty
    /// (unless this call already read it as empty). A `#[near(storage)]` record value is read
    /// first (the unknown-collection check of API round 5, see [`Removable`]).
    #[track_caller]
    pub fn remove(&mut self)
    where
        T: Removable,
    {
        self.path.live();
        Self::clear_children(self.p());
    }
}

impl<'a, T: Value> At<'a, Lazy<T>> {
    read_only! {
        set(T);
        get_mut();
        remove();
    }
    #[track_caller]
    pub fn get(&self) -> Option<&'a T> {
        cache::get::<T>(self.inner.p()).map(|p| unsafe { r(p) })
    }
}

// ---------------------------------------------------------------------------------------
// LookupSet / Set
// ---------------------------------------------------------------------------------------

/// `LookupMap<K, ()>`.
#[derive(Debug)]
pub struct LookupSet<K>(LookupMap<K, ()>);
impl<K> Default for LookupSet<K> {
    fn default() -> Self {
        LookupSet(LookupMap::default())
    }
}
impl<K: StorageKey + 'static> Value for LookupSet<K> {
    const INLINE: bool = false;
    const KIND: u8 = super::record::marker::LOOKUP_SET;
    const __ROOT_ONLY: bool = true;
    const MAX_SUBPATH: usize = <LookupMap<K, ()> as Value>::MAX_SUBPATH;
    not_inline_value!(0.path);
}
impl<K: StorageKey + 'static> LookupSet<K> {
    /// 1 `storage_has_key`.
    pub fn contains(&self, k: &K) -> bool {
        self.0.contains_key(k)
    }
    /// 1 write.
    pub fn insert(&mut self, k: K) {
        self.0.insert(k, ())
    }
    /// 1 remove, no read, issued even when the key is absent (a blind remove, as
    /// [`LookupMap::discard`]).
    pub fn remove(&mut self, k: &K) {
        self.0.discard(k)
    }
}
impl<K: StorageKey + 'static> At<'_, LookupSet<K>> {
    read_only! {
        insert(K);
        remove(&K);
    }
    pub fn contains(&self, k: &K) -> bool {
        self.inner.contains(k)
    }
}

/// `Map<K, ()>`.
#[derive(Debug)]
pub struct Set<K>(Map<K, ()>);
impl<K> Default for Set<K> {
    fn default() -> Self {
        Set(Map::default())
    }
}
impl<K: StorageKey + Clone + 'static> Value for Set<K> {
    const INLINE: bool = false;
    const KIND: u8 = super::record::marker::SET;
    const MAX_SUBPATH: usize = <Map<K, ()> as Value>::MAX_SUBPATH;
    not_inline_value!(0.path);
}
impl<K: StorageKey + Clone + 'static> Removable for Set<K> {
    fn clear_children(path: &Path) {
        <Map<K, ()> as Removable>::clear_children(path)
    }
    fn clear_children_some(path: &Path, budget: &mut u32) -> bool {
        <Map<K, ()> as Removable>::clear_children_some(path, budget)
    }
}
impl<K: StorageKey + Clone + 'static> Set<K> {
    /// 1 `storage_has_key` unless cached.
    pub fn contains(&self, k: &K) -> bool {
        self.0.contains_key(k)
    }
    /// Returns `true` if the key was new. New: 1 index read, 1 header read, 4 writes;
    /// existing: 1 index read, nothing written.
    pub fn insert(&mut self, k: K) -> bool {
        if cache::get::<u32>(&self.0.index_path(&k)).is_some() {
            return false;
        }
        self.0.insert(k, ())
    }
    /// Returns `true` if the key was present. No entry read.
    pub fn remove(&mut self, k: &K) -> bool {
        self.0.discard(k)
    }
    /// 1 read.
    pub fn len(&self) -> u32 {
        self.0.len()
    }
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
    /// The `i`th key in key-vector order: 1 read.
    pub fn key_at(&self, i: u32) -> Option<K> {
        self.0.key_at(i)
    }
    /// Cursor page of keys: as `Map::keys_page`, including its `CursorGone` error.
    pub fn page(&self, after: Option<&K>, limit: u32) -> Result<KeyPage<K>, CursorGone> {
        self.0.keys_page(after, limit)
    }
    /// 1 read per key yielded, plus 1 for the length. `skip(n)` is free, so offset pagination
    /// is `iter().skip(from).take(limit)`. Order is not insertion order after a remove.
    pub fn iter(&self) -> IndexIter<impl FnMut(u32) -> K + '_> {
        self.0.keys()
    }
}
impl<'a, K: StorageKey + Clone + 'static> At<'a, Set<K>> {
    read_only! {
        insert(K);
        remove(&K);
        clear();
    }
    pub fn contains(&self, k: &K) -> bool {
        self.inner.contains(k)
    }
    pub fn len(&self) -> u32 {
        self.inner.len()
    }
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }
    pub fn key_at(&self, i: u32) -> Option<K> {
        self.inner.key_at(i)
    }
    pub fn page(&self, after: Option<&K>, limit: u32) -> Result<KeyPage<K>, CursorGone> {
        self.inner.page(after, limit)
    }
    /// As `Set::iter`; `skip(n)` is free.
    pub fn iter(&self) -> IndexIter<impl FnMut(u32) -> K + use<'a, K>> {
        At::<'a, Map<K, ()>>::new(Map::__bound_at(self.inner.0.p().clone())).keys()
    }
}

// ---------------------------------------------------------------------------------------
// GroupedMap
// ---------------------------------------------------------------------------------------

/// **Deferred out of the first release (D3, Oct 17): hidden from docs, kept compiling for the
/// prototype's NFT test. Do not use in new code.** Write `Map<G, R>` where `R` is a
/// `#[near(storage)]` record holding `Map<K, V>` (and the group's scalars). An alternative to
/// revisit later: allow a collection directly as a map value.
///
/// One `Map` per group, at `path ++ 00 ++ h16(g) ++ borsh(g)`. Groups are not iterated, so a
/// `GroupedMap` is not `Removable`.
#[doc(hidden)]
///
/// `GroupedMap<G, K, ()>` is a grouped set (tokens per owner, members per role) at no extra
/// cost: `Map::remove` on a zero-sized value skips the entry read, and `group(&g).keys()` /
/// `keys().skip(from).take(n)` / `keys_page` enumerate one group.
pub struct GroupedMap<G, K, V> {
    path: Binding,
    _m: PhantomData<fn() -> (G, K, V)>,
}
collection_value!(GroupedMap<G, K, V>);
impl<G: StorageKey + 'static, K: StorageKey + Clone + 'static, V: Value> Value
    for GroupedMap<G, K, V>
{
    const INLINE: bool = false;
    const KIND: u8 = super::record::marker::GROUPED_MAP;
    const __ROOT_ONLY: bool = true;
    const MAX_SUBPATH: usize = 1 + ENTRY_HASH_LEN + G::MAX_LEN + <Map<K, V> as Value>::MAX_SUBPATH;
    not_inline_value!(path);
}

/// A group's `Map`, read-only, with reads living as long as the `GroupedMap` borrow.
#[doc(hidden)]
pub type Group<'a, K, V> = At<'a, Map<K, V>>;

/// A group's `Map`, mutably borrowed from the `GroupedMap` for `'a`. A plain `&mut`, so
/// `let o = orders.group_mut(&m).get_mut(&k);` borrows from the `GroupedMap`, not a
/// temporary. The handle lives in the per-call cache arena until the flush (invariant 4).
#[doc(hidden)]
pub type GroupMut<'a, K, V> = &'a mut Map<K, V>;

impl<G: StorageKey + 'static, K: StorageKey + Clone + 'static, V: Value> GroupedMap<G, K, V> {
    #[track_caller]
    fn group_path(&self, g: &G) -> Path {
        bound(&self.path).kind(kind::ENTRIES).entry(g)
    }
    #[track_caller]
    pub fn group(&self, g: &G) -> Group<'_, K, V> {
        At::new(Map::__bound_at(self.group_path(g)))
    }
    #[track_caller]
    pub fn group_mut(&mut self, g: &G) -> GroupMut<'_, K, V> {
        self.path.live();
        // SAFETY: `&mut self` for `'_` makes this the only `&mut` reachable from this
        // `GroupedMap` during the borrow.
        unsafe { arena_mut(Map::__bound_at(self.group_path(g))) }
    }
}

impl<'a, G: StorageKey + 'static, K: StorageKey + Clone + 'static, V: Value>
    At<'a, GroupedMap<G, K, V>>
{
    read_only! {
        group_mut(&G);
    }
    #[track_caller]
    pub fn group(&self, g: &G) -> Group<'a, K, V> {
        At::new(Map::__bound_at(self.inner.group_path(g)))
    }
}

// ---------------------------------------------------------------------------------------
// View fields: collections appear as `At` handles
// ---------------------------------------------------------------------------------------

macro_rules! view_as_at {
    ($ty:ident < $($g:ident),* > where $($b:tt)*) => {
        impl<'a, R: Value, $($g),*> ViewField<'a, R> for $ty<$($g),*>
        where $($b)*
        {
            type Out = At<'a, $ty<$($g),*>>;
            fn __view_field(field_path: Path) -> Self::Out {
                At::new(Self::__bound_at(field_path))
            }
        }
    };
}
view_as_at!(LookupMap<K, V> where K: StorageKey + 'static, V: Value);
view_as_at!(Map<K, V> where K: StorageKey + Clone + 'static, V: Value);
view_as_at!(Vector<T> where T: Value);
view_as_at!(Lazy<T> where T: Value);
view_as_at!(GroupedMap<G, K, V> where G: StorageKey + 'static, K: StorageKey + Clone + 'static, V: Value);
impl<'a, R: Value, K: StorageKey + 'static> ViewField<'a, R> for LookupSet<K> {
    type Out = At<'a, LookupSet<K>>;
    fn __view_field(field_path: Path) -> Self::Out {
        At::new(LookupSet(LookupMap::__bound_at(field_path)))
    }
}
impl<'a, R: Value, K: StorageKey + Clone + 'static> ViewField<'a, R> for Set<K> {
    type Out = At<'a, Set<K>>;
    fn __view_field(field_path: Path) -> Self::Out {
        At::new(Set(Map::__bound_at(field_path)))
    }
}

// ---------------------------------------------------------------------------------------
// Mutable view fields: collections appear as `&'a mut` handles from the cache arena
// ---------------------------------------------------------------------------------------

// The header-bearing collections and `Lazy` remember the record that stores them (round 7, the
// A3 first-write rule). The root-only ones (`LookupMap`, `LookupSet`, `GroupedMap`) never sit in
// a record (the derive refuses them there), so their handles carry no anchor.
macro_rules! view_mut_as_arena {
    ($ty:ty, [$($g:tt)*], $make:expr) => {
        impl<'a, R: Value, $($g)*> ViewFieldMut<'a, R> for $ty {
            type Out = &'a mut $ty;
            fn __view_field_mut(field_path: Path, anchor: usize) -> Self::Out {
                let make: fn(Path, usize) -> $ty = $make;
                // SAFETY: `at_mut` holds `&'a mut` on the parent collection, every field has a
                // distinct path, and the handle lives in the arena until the flush.
                unsafe { arena_mut(make(field_path, anchor)) }
            }
        }
    };
}
view_mut_as_arena!(LookupMap<K, V>, [K: StorageKey + 'static, V: Value], |p, _| LookupMap::__bound_at(p));
view_mut_as_arena!(Map<K, V>, [K: StorageKey + Clone + 'static, V: Value], |p, a| {
    Map::bound_anchored(p, a, super::record::marker::MAP)
});
view_mut_as_arena!(Vector<T>, [T: Value], Vector::bound_anchored);
view_mut_as_arena!(Lazy<T>, [T: Value], Lazy::bound_anchored);
view_mut_as_arena!(
    GroupedMap<G, K, V>,
    [G: StorageKey + 'static, K: StorageKey + Clone + 'static, V: Value],
    |p, _| GroupedMap::__bound_at(p)
);
view_mut_as_arena!(LookupSet<K>, [K: StorageKey + 'static], |p, _| LookupSet(LookupMap::__bound_at(p)));
view_mut_as_arena!(Set<K>, [K: StorageKey + Clone + 'static], |p, a| {
    Set(Map::bound_anchored(p, a, super::record::marker::SET))
});
