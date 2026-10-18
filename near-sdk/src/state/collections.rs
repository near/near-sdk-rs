//! The v2 collection family. Every handle is a path; every byte goes through the cache.
//!
//! Handles are not `Clone`: two handles to one path would break the cache's aliasing
//! invariant (see `cache.rs`).

use super::cache;
use super::key::{ENTRY_HASH_LEN, Path, StorageKey, kind};
use super::record::RecordError;
use super::value::{Leaf, Record, Removable, Value, ViewField, ViewFieldMut};
use core::marker::PhantomData;
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

/// Where a collection handle points.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
enum Binding {
    /// Constructed with `Default` and not yet placed.
    #[default]
    Unbound,
    Bound(Path),
    /// The record holding this collection was removed; its children were cleared.
    Detached,
}

#[track_caller]
fn bound(b: &Binding) -> &Path {
    match b {
        Binding::Bound(p) => p,
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
        Binding::Bound(old) if old != path => panic!(
            "state: collection already bound to {old:?}; a record cannot move to another key \
             (`mem::take`/`mem::replace` on a stored record is unsupported; remove it and build \
             a new one)"
        ),
        Binding::Detached => panic!(
            "state: this record was removed and its collections cleared; build a new record \
             instead of re-inserting the removed one"
        ),
        _ => *slot = Binding::Bound(path.clone()),
    }
}

fn not_inline<T>() -> Result<T, RecordError> {
    Err(RecordError::Leaf("a collection has no inline encoding".into()))
}

fn len_at(header: &Path) -> u32 {
    cache::get::<u32>(header).map(|p| unsafe { *r(p) }).unwrap_or(0)
}

fn set_len_at(header: &Path, n: u32) {
    if n == 0 {
        cache::delete::<u32>(header);
    } else {
        cache::put(header, n);
    }
}

/// `at_mut`: 1 read of the record's bytes (no decode) unless cached; `None` if the record is
/// absent (so it never creates children of a record that does not exist). The read, rather
/// than a `storage_has_key`, is what lets it see the D9 clearing flag; it panics on a record
/// being cleared.
#[track_caller]
fn record_at_mut<'a, V: Record>(path: Path) -> Option<V::ViewMut<'a>> {
    let clearing = cache::record_state(&path)?;
    if clearing && V::MAX_SUBPATH > 0 {
        clearing_panic(&path);
    }
    Some(V::__view_mut(path))
}

#[track_caller]
fn clearing_panic(path: &Path) -> ! {
    panic!(
        "state: the record at {path:?} is being cleared by `remove_some`; writes to it or below \
         it are refused until clearing completes (call `remove_some` again until it returns \
         false)"
    )
}

/// D9: panics if the record at `path` (already read) is being cleared. Only a value that owns
/// storage can be (a Borsh leaf may legitimately start with `00 00`).
#[track_caller]
fn assert_not_clearing<V: Value>(path: &Path) {
    if V::MAX_SUBPATH > 0 && cache::is_clearing(path) {
        clearing_panic(path);
    }
}

/// Moves a bound handle into the per-call arena and returns `&'a mut` to it.
///
/// SAFETY (for callers): `'a` must be bounded by a `&mut` borrow of the parent collection,
/// and the handle is freed at the flush (invariant 4 in `cache.rs`).
unsafe fn arena_mut<'a, C: core::any::Any>(c: C) -> &'a mut C {
    unsafe { &mut *cache::arena(c).as_ptr() }
}

/// Panics if `V` owns collections and a value already exists at `path`.
#[track_caller]
fn assert_insert_new<V: Value>(path: &Path) {
    if V::MAX_SUBPATH > 0 && cache::exists(path) {
        panic!("state: record exists; use get_mut, or remove it first");
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
impl core::fmt::Display for CursorGone {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(Self::MSG)
    }
}
impl AsRef<str> for CursorGone {
    fn as_ref(&self) -> &str {
        Self::MSG
    }
}
impl std::error::Error for CursorGone {}

/// One page of a cursor-paginated collection.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(crate = "crate::serde")]
pub struct Page<K, V> {
    pub items: std::vec::Vec<(K, V)>,
    pub next: Option<K>,
}

/// One page of keys.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(crate = "crate::serde")]
pub struct KeyPage<K> {
    pub keys: std::vec::Vec<K>,
    pub next: Option<K>,
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

macro_rules! collection_value {
    ($ty:ident < $($g:ident),* >) => {
        impl<$($g),*> Default for $ty<$($g),*> {
            fn default() -> Self {
                Self { path: Binding::Unbound, _m: PhantomData }
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
                    Binding::Bound(p) => Some(p),
                    _ => None,
                }
            }
            #[doc(hidden)]
            pub fn __bound_at(path: Path) -> Self {
                Self { path: Binding::Bound(path), _m: PhantomData }
            }
        }
    };
}

macro_rules! not_inline_value {
    () => {
        fn encode_value(&self, _: Option<&[u8]>) -> std::vec::Vec<u8> {
            unreachable!("collections are not stored inline")
        }
        fn decode_value(_: &[u8], _: &Path) -> Result<Self, RecordError> {
            not_inline()
        }
        fn bind(&mut self, path: &Path) {
            bind_slot(&mut self.path, path)
        }
        fn detach(&mut self) {
            self.path = Binding::Detached;
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
    const MAX_SUBPATH: usize = 1 + ENTRY_HASH_LEN + K::MAX_LEN + V::MAX_SUBPATH;
    not_inline_value!();
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
        cache::get::<V>(&self.entry_path(k)).map(|p| unsafe { r(p) })
    }

    /// 1 `storage_has_key` (no value bytes) unless the key is already cached.
    #[track_caller]
    pub fn contains_key(&self, k: &K) -> bool {
        cache::exists(&self.entry_path(k))
    }

    /// 1 read; 1 write at flush if the value changed. Unknown tags are preserved.
    #[track_caller]
    pub fn get_mut(&mut self, k: &K) -> Option<&mut V> {
        let path = self.entry_path(k);
        let p = cache::get::<V>(&path)?;
        assert_not_clearing::<V>(&path);
        cache::touch(&path);
        Some(unsafe { m(p) })
    }

    /// Always overwrites: 1 write, no read, replaces the stored value whole (unknown tags
    /// included). Returns nothing, since knowing whether the key existed would cost a read.
    ///
    /// Only for values that own no collections (`V: Leaf`). For a record that owns collections
    /// use [`LookupMap::insert_new`] or `entry(k).or_default()`:
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
    ///     // note: use `insert_new(k, v)` (panics if the key exists, returns the bound record)
    ///     // or `entry(k).or_default()`; to replace a record, `remove` it first
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
    ///     let m = markets.insert_new(7, Market::default());
    ///     m.orders.insert(1, 100);
    /// }
    /// ```
    #[track_caller]
    pub fn insert(&mut self, k: K, mut v: V)
    where
        V: Leaf,
    {
        let path = self.entry_path(&k);
        v.bind(&path);
        cache::put(&path, v);
    }

    /// Inserts a record that may own collections, and returns it bound to its key:
    /// 1 `storage_has_key` + 1 write. Panics if the key exists ("record exists; use get_mut,
    /// or remove it first"), so first-write-wins needs no separate check.
    #[track_caller]
    pub fn insert_new(&mut self, k: K, mut v: V) -> &mut V
    where
        V: Record,
    {
        let path = self.entry_path(&k);
        if cache::exists(&path) {
            panic!("state: record exists; use get_mut, or remove it first");
        }
        v.bind(&path);
        unsafe { m(cache::put(&path, v)) }
    }

    /// 1 read, 1 write. The old value comes back detached, its collections cleared.
    #[track_caller]
    pub fn replace(&mut self, k: K, mut v: V) -> Option<V>
    where
        V: Removable,
    {
        let path = self.entry_path(&k);
        let mut old = None;
        if cache::get::<V>(&path).is_some() {
            V::clear_children(&path);
            let mut o = cache::delete::<V>(&path).expect("value was just read");
            o.detach();
            old = Some(o);
        }
        v.bind(&path);
        cache::put(&path, v);
        old
    }

    /// 1 read, 1 remove, plus clearing the value's collections. The returned record is
    /// detached: its collections panic on use.
    #[track_caller]
    pub fn remove(&mut self, k: &K) -> Option<V>
    where
        V: Removable,
    {
        let path = self.entry_path(k);
        cache::get::<V>(&path)?;
        V::clear_children(&path);
        let mut v = cache::delete::<V>(&path).expect("value was just read");
        v.detach();
        Some(v)
    }

    /// 1 remove, no read, plus clearing the value's collections.
    #[track_caller]
    pub fn discard(&mut self, k: &K)
    where
        V: Removable,
    {
        let path = self.entry_path(k);
        V::clear_children(&path);
        cache::delete::<V>(&path);
    }

    /// Removes the record at `k` in bounded steps (D9): each call removes at most `n` entries
    /// of the collections it owns, then the record itself once they are empty. Returns `true`
    /// while entries remain (call again), `false` once the record is gone (or was absent).
    ///
    /// While entries remain the record carries the clearing flag (2 bytes, `00 00`, in its
    /// record bytes), so `get_mut`, `entry` and `at_mut` on it panic in every later call until
    /// clearing completes: nothing can be added below a record faster than it is cleared.
    /// `get` / `at` still read it. Cost per call: 1 read of the record, then what clearing `n`
    /// entries costs, plus 1 record write (the flag) on a call that does not finish.
    #[track_caller]
    pub fn remove_some(&mut self, k: &K, n: u32) -> bool
    where
        V: Record + Removable,
    {
        let path = self.entry_path(k);
        if cache::get::<V>(&path).is_none() {
            return false;
        }
        let mut budget = n;
        if V::clear_children_some(&path, &mut budget) {
            self.discard(k);
            false
        } else {
            cache::set_clearing(&path);
            true
        }
    }

    #[track_caller]
    pub fn entry(&mut self, k: K) -> LookupMapEntry<'_, K, V> {
        let path = self.entry_path(&k);
        LookupMapEntry { path, _m: PhantomData }
    }

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
    /// accessors return `Option`).
    #[track_caller]
    pub fn at_mut(&mut self, k: &K) -> Option<V::ViewMut<'_>>
    where
        V: Record,
    {
        record_at_mut::<V>(self.entry_path(k))
    }
}

impl<'a, K: StorageKey + 'static, V: Value> At<'a, LookupMap<K, V>> {
    read_only! {
        insert(K, V);
        insert_new(K, V);
        remove_some(&K, u32);
        remove(&K);
        discard(&K);
        get_mut(&K);
        entry(K);
        at_mut(&K);
    }
    #[track_caller]
    pub fn get(&self, k: &K) -> Option<&'a V> {
        cache::get::<V>(&self.inner.entry_path(k)).map(|p| unsafe { r(p) })
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
}

pub struct LookupMapEntry<'a, K, V> {
    path: Path,
    _m: PhantomData<(&'a mut LookupMap<K, V>, K)>,
}

impl<'a, K, V: Value> LookupMapEntry<'a, K, V> {
    /// 1 read; if absent, 1 write at flush.
    #[track_caller]
    pub fn or_insert_with(self, f: impl FnOnce() -> V) -> &'a mut V {
        if let Some(p) = cache::get::<V>(&self.path) {
            assert_not_clearing::<V>(&self.path);
            cache::touch(&self.path);
            return unsafe { m(p) };
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
    const MAX_SUBPATH: usize = 1 + ENTRY_HASH_LEN + K::MAX_LEN + V::MAX_SUBPATH;
    not_inline_value!();
}

impl<K: StorageKey + Clone + 'static, V: Removable> Removable for Map<K, V> {
    fn clear_children(path: &Path) {
        Map::<K, V>::__bound_at(path.clone()).clear();
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

    /// 1 read.
    pub fn len(&self) -> u32 {
        len_at(&self.header())
    }
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// 1 read.
    #[track_caller]
    pub fn get(&self, k: &K) -> Option<&V> {
        cache::get::<V>(&self.entry_path(k)).map(|p| unsafe { r(p) })
    }

    /// 1 `storage_has_key` unless cached.
    #[track_caller]
    pub fn contains_key(&self, k: &K) -> bool {
        cache::exists(&self.entry_path(k))
    }

    /// 1 read; 1 write at flush if the value changed.
    #[track_caller]
    pub fn get_mut(&mut self, k: &K) -> Option<&mut V> {
        let path = self.entry_path(k);
        let p = cache::get::<V>(&path)?;
        assert_not_clearing::<V>(&path);
        cache::touch(&path);
        Some(unsafe { m(p) })
    }

    fn push_key(&mut self, k: &K) {
        let len = self.len();
        cache::put(&self.index_path(k), len);
        cache::put(&self.slot_path(len), k.clone());
        set_len_at(&self.header(), len.checked_add(1).expect("Map is full"));
    }

    /// Always overwrites. Returns `true` if the key was new (free: the index read says so).
    ///
    /// New key: 1 index read, 1 header read, 4 writes. Existing key: 1 index read, 1 write.
    /// First-write-wins is `entry(k).or_insert(v)`, or `insert_new` for a record.
    ///
    /// Only for values that own no collections (`V: Leaf`); see [`LookupMap::insert`] for the
    /// rejected call and the fix. For a record that owns collections use [`Map::insert_new`] or
    /// `entry(k).or_default()`.
    #[track_caller]
    pub fn insert(&mut self, k: K, mut v: V) -> bool
    where
        V: Leaf,
    {
        let path = self.entry_path(&k);
        let exists = cache::get::<u32>(&self.index_path(&k)).is_some();
        v.bind(&path);
        cache::put(&path, v);
        if !exists {
            self.push_key(&k);
        }
        !exists
    }

    /// Inserts a record that may own collections, and returns it bound to its key. New key:
    /// 1 index read, 1 header read, 4 writes. Panics if the key exists ("record exists; use
    /// get_mut, or remove it first"): an existing record's collections would otherwise be
    /// left behind or merged.
    #[track_caller]
    pub fn insert_new(&mut self, k: K, mut v: V) -> &mut V
    where
        V: Record,
    {
        let path = self.entry_path(&k);
        if cache::get::<u32>(&self.index_path(&k)).is_some() {
            panic!("state: record exists; use get_mut, or remove it first");
        }
        v.bind(&path);
        let p = cache::put(&path, v);
        self.push_key(&k);
        unsafe { m(p) }
    }

    #[track_caller]
    pub fn entry(&mut self, k: K) -> MapEntry<'_, K, V> {
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
            cache::get::<V>(&path)?;
        }
        let i = unsafe { *r(cache::get::<u32>(&idx_path)?) };
        let len = self.len();
        V::clear_children(&path);
        if let Some(mut v) = cache::delete::<V>(&path) {
            v.detach();
            value = Some(v);
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
        set_len_at(&self.header(), last);
        Some(value)
    }

    /// 1 entry read, 1 index read, 1 header read, 2 removes (entry, index), 1 read and 1
    /// remove of the last slot, 2 writes for the moved key, 1 header write (a remove when
    /// the map empties). If the key is last: no moved key.
    ///
    /// A zero-sized `V` (such as `()`) skips the entry read: the index read already says
    /// whether the key exists, and the value carries no bytes. So `Map<K, ()>` and
    /// `GroupedMap<G, K, ()>` remove at the cost of `discard`.
    #[track_caller]
    pub fn remove(&mut self, k: &K) -> Option<V>
    where
        V: Removable,
    {
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

    /// As `remove` without the entry read.
    #[track_caller]
    pub fn discard(&mut self, k: &K) -> bool
    where
        V: Removable,
    {
        self.remove_inner(k, false).is_some()
    }

    fn key_at_ptr(&self, i: u32) -> Option<NonNull<K>> {
        cache::get::<K>(&self.slot_path(i))
    }

    /// The `i`th key in key-vector order: 1 read.
    pub fn key_at(&self, i: u32) -> Option<K> {
        self.key_at_ptr(i).map(|p| unsafe { r(p) }.clone())
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
    /// whose key was removed fails loudly; offset paging (`iter().skip(from).take(n)`) does
    /// not fail but can SKIP a key: a removal behind the offset moves the last key into the
    /// hole, ahead of where the next page starts. An on-chain batch job must not page a map
    /// that changes between its calls.
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
        let len = self.len();
        let end = start.saturating_add(limit).min(len);
        let mut items = std::vec::Vec::with_capacity((end.saturating_sub(start)) as usize);
        for i in start..end {
            let k = self.key_at(i).expect("dense key vector");
            let v = f(&k, self.get(&k).expect("indexed key has an entry"));
            items.push((k, v));
        }
        let next = if end < len { items.last().map(|(k, _)| k.clone()) } else { None };
        Ok(Page { items, next })
    }

    /// Key-only cursor page: as `page`, 1 read per key; same `CursorGone` error.
    pub fn keys_page(&self, after: Option<&K>, limit: u32) -> Result<KeyPage<K>, CursorGone> {
        let start = self.cursor_start(after)?;
        let len = self.len();
        let end = start.saturating_add(limit).min(len);
        let keys: std::vec::Vec<K> =
            (start..end).map(|i| self.key_at(i).expect("dense key vector")).collect();
        let next = if end < len { keys.last().cloned() } else { None };
        Ok(KeyPage { keys, next })
    }

    /// 2 reads per entry yielded, plus 1 for the length. Order is the key vector's: insertion
    /// order until a remove moves the last key into the hole. `skip(n)` is free, so offset
    /// pagination is `iter().skip(from).take(limit)`.
    pub fn iter<'s>(&'s self) -> IndexIter<impl FnMut(u32) -> (K, &'s V) + 's> {
        IndexIter::new(self.len(), move |i| {
            let k = self.key_at(i).expect("dense key vector");
            let v = cache::get::<V>(&self.entry_path(&k)).expect("indexed key has an entry");
            (k, unsafe { r(v) })
        })
    }

    /// 1 read per key yielded, plus 1 for the length. `skip(n)` is free.
    pub fn keys(&self) -> IndexIter<impl FnMut(u32) -> K + '_> {
        IndexIter::new(self.len(), move |i| self.key_at(i).expect("dense key vector"))
    }

    /// Removes everything: 1 header read, then per entry 1 slot read and 3 removes (entry,
    /// index, slot), plus clearing each value's own collections; then 1 header remove.
    pub fn clear(&mut self)
    where
        V: Removable,
    {
        while self.clear_some(u32::MAX) {}
    }

    /// Removes at most `n` entries from the end. Returns `true` if entries remain.
    pub fn clear_some(&mut self, n: u32) -> bool
    where
        V: Removable,
    {
        let len = self.len();
        let stop = len.saturating_sub(n);
        for i in (stop..len).rev() {
            let slot = self.slot_path(i);
            let k: K = unsafe { r(cache::get::<K>(&slot).expect("dense key vector")) }.clone();
            let path = self.entry_path(&k);
            V::clear_children(&path);
            cache::delete::<V>(&path);
            cache::delete::<u32>(&self.index_path(&k));
            cache::delete::<K>(&slot);
        }
        set_len_at(&self.header(), stop);
        stop > 0
    }

    /// Bounded clear from the end (D9), clearing each value's own collections first; see
    /// [`Removable::clear_children_some`].
    fn clear_budgeted(&mut self, budget: &mut u32) -> bool
    where
        V: Removable,
    {
        loop {
            let len = self.len();
            if len == 0 {
                return true;
            }
            let i = len - 1;
            let slot = self.slot_path(i);
            let k: K = unsafe { r(cache::get::<K>(&slot).expect("dense key vector")) }.clone();
            let path = self.entry_path(&k);
            if !V::clear_children_some(&path, budget) || *budget == 0 {
                return false;
            }
            *budget -= 1;
            cache::delete::<V>(&path);
            cache::delete::<u32>(&self.index_path(&k));
            cache::delete::<K>(&slot);
            set_len_at(&self.header(), i);
        }
    }

    /// Removes the record at `k` in bounded steps (D9): as [`LookupMap::remove_some`], and
    /// the key leaves the key vector in the call that finishes. Returns `true` while entries
    /// remain.
    #[track_caller]
    pub fn remove_some(&mut self, k: &K, n: u32) -> bool
    where
        V: Record + Removable,
    {
        let path = self.entry_path(k);
        if cache::get::<V>(&path).is_none() {
            return false;
        }
        let mut budget = n;
        if V::clear_children_some(&path, &mut budget) {
            self.remove_inner(k, false);
            false
        } else {
            cache::set_clearing(&path);
            true
        }
    }

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
    /// accessors return `Option`).
    #[track_caller]
    pub fn at_mut(&mut self, k: &K) -> Option<V::ViewMut<'_>>
    where
        V: Record,
    {
        record_at_mut::<V>(self.entry_path(k))
    }
}

impl<'a, K: StorageKey + Clone + 'static, V: Value> At<'a, Map<K, V>> {
    read_only! {
        insert(K, V);
        insert_new(K, V);
        remove_some(&K, u32);
        remove(&K);
        discard(&K);
        get_mut(&K);
        entry(K);
        at_mut(&K);
        clear();
    }
    #[track_caller]
    pub fn get(&self, k: &K) -> Option<&'a V> {
        cache::get::<V>(&self.inner.entry_path(k)).map(|p| unsafe { r(p) })
    }
    #[track_caller]
    pub fn contains_key(&self, k: &K) -> bool {
        self.inner.contains_key(k)
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
            let v = cache::get::<V>(&map.entry_path(&k)).expect("indexed key has an entry");
            (k, unsafe { r(v) })
        })
    }
    /// As `Map::keys`; `skip(n)` is free.
    pub fn keys(&self) -> IndexIter<impl FnMut(u32) -> K + use<'a, K, V>> {
        let map = Map::<K, V>::__bound_at(self.inner.p().clone());
        IndexIter::new(map.len(), move |i| map.key_at(i).expect("dense key vector"))
    }
    #[track_caller]
    pub fn at(&self, k: &K) -> V::View<'a>
    where
        V: Record,
    {
        V::__view(self.inner.entry_path(k))
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
        if let Some(p) = cache::get::<V>(&path) {
            assert_not_clearing::<V>(&path);
            cache::touch(&path);
            return unsafe { m(p) };
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
    const MAX_SUBPATH: usize = 1 + 4 + T::MAX_SUBPATH;
    not_inline_value!();
}

impl<T: Removable> Removable for Vector<T> {
    fn clear_children(path: &Path) {
        Vector::<T>::__bound_at(path.clone()).clear();
    }
    fn clear_children_some(path: &Path, budget: &mut u32) -> bool {
        let v = Vector::<T>::__bound_at(path.clone());
        loop {
            let len = v.len();
            if len == 0 {
                return true;
            }
            let slot = v.slot(len - 1);
            if !T::clear_children_some(&slot, budget) || *budget == 0 {
                return false;
            }
            *budget -= 1;
            cache::delete::<T>(&slot);
            set_len_at(&v.header(), len - 1);
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

    /// 1 read.
    pub fn len(&self) -> u32 {
        len_at(&self.header())
    }
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// 1 read (bounds are not checked against the header; a missing slot is `None`).
    pub fn get(&self, i: u32) -> Option<&T> {
        cache::get::<T>(&self.slot(i)).map(|p| unsafe { r(p) })
    }

    pub fn get_mut(&mut self, i: u32) -> Option<&mut T> {
        let path = self.slot(i);
        let p = cache::get::<T>(&path)?;
        cache::touch(&path);
        Some(unsafe { m(p) })
    }

    /// 1 header read, 1 write, 1 header write.
    pub fn push(&mut self, mut v: T) {
        let len = self.len();
        let path = self.slot(len);
        v.bind(&path);
        cache::put(&path, v);
        set_len_at(&self.header(), len + 1);
    }

    /// 1 header read, 1 slot read (the returned value), 1 remove, 1 header write.
    pub fn pop(&mut self) -> Option<T>
    where
        T: Removable,
    {
        let len = self.len();
        if len == 0 {
            return None;
        }
        let path = self.slot(len - 1);
        cache::get::<T>(&path);
        T::clear_children(&path);
        let mut v = cache::delete::<T>(&path);
        if let Some(v) = v.as_mut() {
            v.detach();
        }
        set_len_at(&self.header(), len - 1);
        v
    }

    /// 1 header read, 2 reads (slot `i`, last), 1 write, 1 remove, 1 header write.
    #[track_caller]
    pub fn swap_remove(&mut self, i: u32) -> T {
        let () = Self::CHECK_MOVABLE;
        let len = self.len();
        assert!(i < len, "state::Vector index {i} out of bounds (len {len})");
        let path = self.slot(i);
        cache::get::<T>(&path).expect("dense");
        let v = cache::delete::<T>(&path).expect("just read");
        let last = len - 1;
        if i != last {
            let last_path = self.slot(last);
            cache::get::<T>(&last_path).expect("dense");
            let moved = cache::delete::<T>(&last_path).expect("just read");
            cache::put(&path, moved);
        }
        set_len_at(&self.header(), last);
        v
    }

    /// 1 header read, then 1 remove per element (no element reads), plus clearing each
    /// element's own collections; 1 header remove.
    pub fn clear(&mut self)
    where
        T: Removable,
    {
        let len = self.len();
        for i in 0..len {
            let path = self.slot(i);
            T::clear_children(&path);
            cache::delete::<T>(&path);
        }
        set_len_at(&self.header(), 0);
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
pub struct Lazy<T> {
    path: Binding,
    pending: core::cell::Cell<Option<T>>,
}

impl<T> Default for Lazy<T> {
    fn default() -> Self {
        Lazy { path: Binding::Unbound, pending: core::cell::Cell::new(None) }
    }
}
impl<T> core::fmt::Debug for Lazy<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Lazy({:?})", self.path)
    }
}
impl<T> Lazy<T> {
    /// A `Lazy` holding `v`, written when its record is placed. See the type docs.
    pub fn new(v: T) -> Self {
        Lazy { path: Binding::Unbound, pending: core::cell::Cell::new(Some(v)) }
    }
    /// The path this handle is bound to, if any.
    pub fn path(&self) -> Option<&Path> {
        match &self.path {
            Binding::Bound(p) => Some(p),
            _ => None,
        }
    }
    #[doc(hidden)]
    pub fn __bound_at(path: Path) -> Self {
        Lazy { path: Binding::Bound(path), pending: core::cell::Cell::new(None) }
    }
}

impl<T: Value> Value for Lazy<T> {
    const INLINE: bool = false;
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
        if let Some(mut v) = self.pending.take() {
            assert_insert_new::<T>(path);
            v.bind(path);
            cache::put(path, v);
        }
    }
}

impl<T: Removable> Removable for Lazy<T> {
    fn clear_children(path: &Path) {
        T::clear_children(path);
        cache::delete::<T>(path);
    }
    /// The `Lazy` value's own collections count against the budget; the value is not.
    fn clear_children_some(path: &Path, budget: &mut u32) -> bool {
        if !T::clear_children_some(path, budget) {
            return false;
        }
        cache::delete::<T>(path);
        true
    }
}

impl<T: Value> Lazy<T> {
    /// 1 read.
    #[track_caller]
    pub fn get(&self) -> Option<&T> {
        cache::get::<T>(bound(&self.path)).map(|p| unsafe { r(p) })
    }
    #[track_caller]
    pub fn get_mut(&mut self) -> Option<&mut T> {
        let path = bound(&self.path).clone();
        let p = cache::get::<T>(&path)?;
        cache::touch(&path);
        Some(unsafe { m(p) })
    }
    /// 1 write (a record that owns collections: insert-new, as `LookupMap::insert`).
    #[track_caller]
    pub fn set(&mut self, mut v: T) {
        let path = bound(&self.path).clone();
        assert_insert_new::<T>(&path);
        v.bind(&path);
        cache::put(&path, v);
    }
    /// 1 remove.
    #[track_caller]
    pub fn remove(&mut self)
    where
        T: Removable,
    {
        let path = bound(&self.path).clone();
        T::clear_children(&path);
        cache::delete::<T>(&path);
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
        cache::get::<T>(bound(&self.inner.path)).map(|p| unsafe { r(p) })
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
    const MAX_SUBPATH: usize = <LookupMap<K, ()> as Value>::MAX_SUBPATH;
    fn encode_value(&self, _: Option<&[u8]>) -> std::vec::Vec<u8> {
        unreachable!()
    }
    fn decode_value(_: &[u8], _: &Path) -> Result<Self, RecordError> {
        not_inline()
    }
    fn bind(&mut self, path: &Path) {
        self.0.bind(path)
    }
    fn detach(&mut self) {
        self.0.detach()
    }
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
    /// 1 remove.
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
    const MAX_SUBPATH: usize = <Map<K, ()> as Value>::MAX_SUBPATH;
    fn encode_value(&self, _: Option<&[u8]>) -> std::vec::Vec<u8> {
        unreachable!()
    }
    fn decode_value(_: &[u8], _: &Path) -> Result<Self, RecordError> {
        not_inline()
    }
    fn bind(&mut self, path: &Path) {
        self.0.bind(path)
    }
    fn detach(&mut self) {
        self.0.detach()
    }
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
impl<G, K, V> Default for GroupedMap<G, K, V> {
    fn default() -> Self {
        Self { path: Binding::Unbound, _m: PhantomData }
    }
}
impl<G: StorageKey + 'static, K: StorageKey + Clone + 'static, V: Value> Value
    for GroupedMap<G, K, V>
{
    const INLINE: bool = false;
    const MAX_SUBPATH: usize = 1 + ENTRY_HASH_LEN + G::MAX_LEN + <Map<K, V> as Value>::MAX_SUBPATH;
    not_inline_value!();
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
        let p = cache::arena(Map::<K, V>::__bound_at(self.group_path(g)));
        // SAFETY: the handle is a fresh allocation owned by the cache arena and freed only at
        // the flush / `begin_call` (invariant 4). `&mut self` for `'_` makes this the only
        // `&mut` reachable from this `GroupedMap` during the borrow.
        unsafe { &mut *p.as_ptr() }
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

impl<'a, R: Value, G: StorageKey + 'static, K: StorageKey + Clone + 'static, V: Value>
    ViewField<'a, R> for GroupedMap<G, K, V>
{
    type Out = At<'a, GroupedMap<G, K, V>>;
    fn __view_field(field_path: Path) -> Self::Out {
        At::new(GroupedMap { path: Binding::Bound(field_path), _m: PhantomData })
    }
}
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

macro_rules! view_mut_as_arena {
    ($ty:ty, [$($g:tt)*], $make:expr) => {
        impl<'a, R: Value, $($g)*> ViewFieldMut<'a, R> for $ty {
            type Out = &'a mut $ty;
            fn __view_field_mut(field_path: Path) -> Self::Out {
                let make: fn(Path) -> $ty = $make;
                // SAFETY: `at_mut` holds `&'a mut` on the parent collection, every field has a
                // distinct path, and the handle lives in the arena until the flush.
                unsafe { arena_mut(make(field_path)) }
            }
        }
    };
}
view_mut_as_arena!(LookupMap<K, V>, [K: StorageKey + 'static, V: Value], LookupMap::__bound_at);
view_mut_as_arena!(Map<K, V>, [K: StorageKey + Clone + 'static, V: Value], Map::__bound_at);
view_mut_as_arena!(Vector<T>, [T: Value], Vector::__bound_at);
view_mut_as_arena!(Lazy<T>, [T: Value], Lazy::__bound_at);
view_mut_as_arena!(
    GroupedMap<G, K, V>,
    [G: StorageKey + 'static, K: StorageKey + Clone + 'static, V: Value],
    |p| GroupedMap { path: Binding::Bound(p), _m: PhantomData }
);
view_mut_as_arena!(LookupSet<K>, [K: StorageKey + 'static], |p| LookupSet(LookupMap::__bound_at(p)));
view_mut_as_arena!(Set<K>, [K: StorageKey + Clone + 'static], |p| Set(Map::__bound_at(p)));
