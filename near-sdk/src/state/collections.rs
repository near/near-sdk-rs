//! The v2 collection family. Every handle is a path; every byte goes through the cache.
//!
//! Handles are not `Clone`: two handles to one path would break the cache's aliasing
//! invariant (see `cache.rs`).

use super::cache;
use super::key::{ENTRY_HASH_LEN, Path, StorageKey, kind};
use super::record::RecordError;
use super::value::{Record, Removable, Value, ViewField};
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

/// Panics if `V` owns collections and a value already exists at `path`.
#[track_caller]
fn assert_insert_new<V: Value>(path: &Path) {
    if V::MAX_SUBPATH > 0 && cache::exists(path) {
        panic!("state: record exists; use get_mut, or remove it first");
    }
}

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
// Map
// ---------------------------------------------------------------------------------------

/// Lookup-only map. Entries at `path ++ 00 ++ h16(k) ++ borsh(k)`.
pub struct Map<K, V> {
    path: Binding,
    _m: PhantomData<fn() -> (K, V)>,
}
collection_value!(Map<K, V>);

impl<K: StorageKey + 'static, V: Value> Value for Map<K, V> {
    const INLINE: bool = false;
    const MAX_SUBPATH: usize = 1 + ENTRY_HASH_LEN + K::MAX_LEN + V::MAX_SUBPATH;
    not_inline_value!();
}

impl<K: StorageKey + 'static, V: Value> Map<K, V> {
    const CHECK: () = assert!(V::INLINE, "a map value cannot itself be a collection; use GroupedMap");

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
        cache::touch(&path);
        Some(unsafe { m(p) })
    }

    /// Leaf value: 1 write, no read, replaces the stored value whole.
    /// Record that owns collections: insert-new, 1 `storage_has_key` + 1 write, and panics
    /// if the key exists ("record exists; use get_mut, or remove it first").
    #[track_caller]
    pub fn insert(&mut self, k: K, mut v: V) {
        let path = self.entry_path(&k);
        assert_insert_new::<V>(&path);
        v.bind(&path);
        cache::put(&path, v);
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

    #[track_caller]
    pub fn entry(&mut self, k: K) -> MapEntry<'_, K, V> {
        let path = self.entry_path(&k);
        MapEntry { path, _m: PhantomData }
    }

    /// A view bound to the path for `k`. Reads nothing by itself.
    #[track_caller]
    pub fn at(&self, k: &K) -> V::View<'_>
    where
        V: Record,
    {
        V::__view(self.entry_path(k))
    }
}

impl<'a, K: StorageKey + 'static, V: Value> At<'a, Map<K, V>> {
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

pub struct MapEntry<'a, K, V> {
    path: Path,
    _m: PhantomData<(&'a mut Map<K, V>, K)>,
}

impl<'a, K, V: Value> MapEntry<'a, K, V> {
    /// 1 read; if absent, 1 write at flush.
    pub fn or_insert_with(self, f: impl FnOnce() -> V) -> &'a mut V {
        if let Some(p) = cache::get::<V>(&self.path) {
            cache::touch(&self.path);
            return unsafe { m(p) };
        }
        let mut v = f();
        v.bind(&self.path);
        unsafe { m(cache::put(&self.path, v)) }
    }
    pub fn or_insert(self, v: V) -> &'a mut V {
        self.or_insert_with(|| v)
    }
    pub fn or_default(self) -> &'a mut V
    where
        V: Default,
    {
        self.or_insert_with(V::default)
    }
}

// ---------------------------------------------------------------------------------------
// IterMap
// ---------------------------------------------------------------------------------------

/// `Map` plus a dense key vector: entries `00`, index `01`, key slots `02`, header `03`.
pub struct IterMap<K, V> {
    path: Binding,
    _m: PhantomData<fn() -> (K, V)>,
}
collection_value!(IterMap<K, V>);

impl<K: StorageKey + Clone + 'static, V: Value> Value for IterMap<K, V> {
    const INLINE: bool = false;
    const MAX_SUBPATH: usize = 1 + ENTRY_HASH_LEN + K::MAX_LEN + V::MAX_SUBPATH;
    not_inline_value!();
}

impl<K: StorageKey + Clone + 'static, V: Removable> Removable for IterMap<K, V> {
    fn clear_children(path: &Path) {
        IterMap::<K, V>::__bound_at(path.clone()).clear();
    }
}

impl<K: StorageKey + Clone + 'static, V: Value> IterMap<K, V> {
    const CHECK: () = assert!(V::INLINE, "a map value cannot itself be a collection; use GroupedMap");

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
        cache::touch(&path);
        Some(unsafe { m(p) })
    }

    fn push_key(&mut self, k: &K) {
        let len = self.len();
        cache::put(&self.index_path(k), len);
        cache::put(&self.slot_path(len), k.clone());
        set_len_at(&self.header(), len.checked_add(1).expect("IterMap is full"));
    }

    /// New key: 1 index read, 1 header read, 4 writes. Existing key: 1 index read, 1 write
    /// (leaf values only; a record that owns collections panics: insert is insert-new).
    #[track_caller]
    pub fn insert(&mut self, k: K, mut v: V) {
        let path = self.entry_path(&k);
        let exists = cache::get::<u32>(&self.index_path(&k)).is_some();
        if exists && V::MAX_SUBPATH > 0 {
            panic!("state: record exists; use get_mut, or remove it first");
        }
        v.bind(&path);
        cache::put(&path, v);
        if !exists {
            self.push_key(&k);
        }
    }

    #[track_caller]
    pub fn entry(&mut self, k: K) -> IterEntry<'_, K, V> {
        IterEntry { map: self, k }
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
    #[track_caller]
    pub fn remove(&mut self, k: &K) -> Option<V>
    where
        V: Removable,
    {
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

    #[track_caller]
    fn cursor_start(&self, after: Option<&K>) -> u32 {
        match after {
            None => 0,
            Some(k) => {
                let i = cache::get::<u32>(&self.index_path(k)).unwrap_or_else(|| {
                    panic!(
                        "state: page cursor key no longer exists (removed since the last page); \
                         restart from a known key"
                    )
                });
                (unsafe { *r(i) }) + 1
            }
        }
    }

    /// Cursor page: with a cursor, 2 reads (cursor index, header) plus 2 per item.
    /// Without a cursor, 1 read plus 2 per item. Panics if the cursor key no longer exists.
    #[track_caller]
    pub fn page(&self, after: Option<&K>, limit: u32) -> Page<K, V>
    where
        V: Clone,
    {
        let start = self.cursor_start(after);
        let len = self.len();
        let end = start.saturating_add(limit).min(len);
        let mut items = std::vec::Vec::with_capacity((end.saturating_sub(start)) as usize);
        for i in start..end {
            let k = self.key_at(i).expect("dense key vector");
            let v = self.get(&k).expect("indexed key has an entry").clone();
            items.push((k, v));
        }
        let next = if end < len { items.last().map(|(k, _)| k.clone()) } else { None };
        Page { items, next }
    }

    /// Key-only cursor page: as `page`, 1 read per key. With `get`, this is the path for
    /// record-valued maps, which `page` cannot clone.
    #[track_caller]
    pub fn keys_page(&self, after: Option<&K>, limit: u32) -> KeyPage<K> {
        let start = self.cursor_start(after);
        let len = self.len();
        let end = start.saturating_add(limit).min(len);
        let keys: std::vec::Vec<K> =
            (start..end).map(|i| self.key_at(i).expect("dense key vector")).collect();
        let next = if end < len { keys.last().cloned() } else { None };
        KeyPage { keys, next }
    }

    /// 2 reads per entry, plus 1 for the length.
    pub fn iter(&self) -> impl Iterator<Item = (K, &V)> + '_ {
        (0..self.len()).map(move |i| {
            let k = self.key_at(i).expect("dense key vector");
            let v = cache::get::<V>(&self.entry_path(&k)).expect("indexed key has an entry");
            (k, unsafe { r(v) })
        })
    }

    /// 1 read per key, plus 1 for the length.
    pub fn keys(&self) -> impl Iterator<Item = K> + '_ {
        (0..self.len()).map(move |i| self.key_at(i).expect("dense key vector"))
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

    #[track_caller]
    pub fn at(&self, k: &K) -> V::View<'_>
    where
        V: Record,
    {
        V::__view(self.entry_path(k))
    }
}

impl<'a, K: StorageKey + Clone + 'static, V: Value> At<'a, IterMap<K, V>> {
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
    #[track_caller]
    pub fn page(&self, after: Option<&K>, limit: u32) -> Page<K, V>
    where
        V: Clone,
    {
        self.inner.page(after, limit)
    }
    #[track_caller]
    pub fn keys_page(&self, after: Option<&K>, limit: u32) -> KeyPage<K> {
        self.inner.keys_page(after, limit)
    }
    pub fn iter(&self) -> impl Iterator<Item = (K, &'a V)> + 'a {
        let map = IterMap::<K, V>::__bound_at(self.inner.p().clone());
        (0..map.len()).map(move |i| {
            let k = map.key_at(i).expect("dense key vector");
            let v = cache::get::<V>(&map.entry_path(&k)).expect("indexed key has an entry");
            (k, unsafe { r(v) })
        })
    }
    pub fn keys(&self) -> impl Iterator<Item = K> + 'a {
        let map = IterMap::<K, V>::__bound_at(self.inner.p().clone());
        (0..map.len()).map(move |i| map.key_at(i).expect("dense key vector"))
    }
    #[track_caller]
    pub fn at(&self, k: &K) -> V::View<'a>
    where
        V: Record,
    {
        V::__view(self.inner.entry_path(k))
    }
}

pub struct IterEntry<'a, K, V> {
    map: &'a mut IterMap<K, V>,
    k: K,
}

impl<'a, K: StorageKey + Clone + 'static, V: Value> IterEntry<'a, K, V> {
    /// Existing: 1 read. New: 1 entry read, 1 header read, 4 writes (no index read: an absent
    /// entry implies an absent index).
    pub fn or_insert_with(self, f: impl FnOnce() -> V) -> &'a mut V {
        let path = self.map.entry_path(&self.k);
        if let Some(p) = cache::get::<V>(&path) {
            cache::touch(&path);
            return unsafe { m(p) };
        }
        let mut v = f();
        v.bind(&path);
        let p = cache::put(&path, v);
        self.map.push_key(&self.k);
        unsafe { m(p) }
    }
    pub fn or_insert(self, v: V) -> &'a mut V {
        self.or_insert_with(|| v)
    }
    pub fn or_default(self) -> &'a mut V
    where
        V: Default,
    {
        self.or_insert_with(V::default)
    }
}

// ---------------------------------------------------------------------------------------
// Vec
// ---------------------------------------------------------------------------------------

/// Dense slots `00 ++ u32` and a length header `03`.
pub struct Vec<T> {
    path: Binding,
    _m: PhantomData<fn() -> T>,
}
collection_value!(Vec<T>);

impl<T: Value> Value for Vec<T> {
    const INLINE: bool = false;
    const MAX_SUBPATH: usize = 1 + 4 + T::MAX_SUBPATH;
    not_inline_value!();
}

impl<T: Removable> Removable for Vec<T> {
    fn clear_children(path: &Path) {
        Vec::<T>::__bound_at(path.clone()).clear();
    }
}

impl<T: Value> Vec<T> {
    const CHECK: () = assert!(T::INLINE, "a Vec element cannot itself be a collection");
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
        assert!(i < len, "state::Vec index {i} out of bounds (len {len})");
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

    pub fn iter(&self) -> impl Iterator<Item = &T> + '_ {
        (0..self.len()).map(move |i| self.get(i).expect("dense"))
    }
}

impl<'a, T: Value> At<'a, Vec<T>> {
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
pub struct Lazy<T> {
    path: Binding,
    _m: PhantomData<fn() -> T>,
}
collection_value!(Lazy<T>);

impl<T: Value> Value for Lazy<T> {
    const INLINE: bool = false;
    const MAX_SUBPATH: usize = T::MAX_SUBPATH;
    not_inline_value!();
}

impl<T: Removable> Removable for Lazy<T> {
    fn clear_children(path: &Path) {
        T::clear_children(path);
        cache::delete::<T>(path);
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
    /// 1 write (a record that owns collections: insert-new, as `Map::insert`).
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
    #[track_caller]
    pub fn get(&self) -> Option<&'a T> {
        cache::get::<T>(bound(&self.inner.path)).map(|p| unsafe { r(p) })
    }
}

// ---------------------------------------------------------------------------------------
// Set / IterSet
// ---------------------------------------------------------------------------------------

/// `Map<K, ()>`.
#[derive(Debug)]
pub struct Set<K>(Map<K, ()>);
impl<K> Default for Set<K> {
    fn default() -> Self {
        Set(Map::default())
    }
}
impl<K: StorageKey + 'static> Value for Set<K> {
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
impl<K: StorageKey + 'static> Set<K> {
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
impl<K: StorageKey + 'static> At<'_, Set<K>> {
    pub fn contains(&self, k: &K) -> bool {
        self.inner.contains(k)
    }
}

/// `IterMap<K, ()>`.
#[derive(Debug)]
pub struct IterSet<K>(IterMap<K, ()>);
impl<K> Default for IterSet<K> {
    fn default() -> Self {
        IterSet(IterMap::default())
    }
}
impl<K: StorageKey + Clone + 'static> Value for IterSet<K> {
    const INLINE: bool = false;
    const MAX_SUBPATH: usize = <IterMap<K, ()> as Value>::MAX_SUBPATH;
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
impl<K: StorageKey + Clone + 'static> Removable for IterSet<K> {
    fn clear_children(path: &Path) {
        <IterMap<K, ()> as Removable>::clear_children(path)
    }
}
impl<K: StorageKey + Clone + 'static> IterSet<K> {
    pub fn contains(&self, k: &K) -> bool {
        self.0.contains_key(k)
    }
    pub fn insert(&mut self, k: K) {
        self.0.insert(k, ())
    }
    pub fn remove(&mut self, k: &K) -> bool {
        self.0.discard(k)
    }
    pub fn len(&self) -> u32 {
        self.0.len()
    }
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
    pub fn iter(&self) -> impl Iterator<Item = K> + '_ {
        self.0.keys()
    }
}
impl<K: StorageKey + Clone + 'static> At<'_, IterSet<K>> {
    pub fn contains(&self, k: &K) -> bool {
        self.inner.contains(k)
    }
    pub fn len(&self) -> u32 {
        self.inner.len()
    }
}

// ---------------------------------------------------------------------------------------
// GroupedMap
// ---------------------------------------------------------------------------------------

/// One `IterMap` per group, at `path ++ 00 ++ h16(g) ++ borsh(g)`. Groups are not iterated.
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
    const MAX_SUBPATH: usize = 1 + ENTRY_HASH_LEN + G::MAX_LEN + <IterMap<K, V> as Value>::MAX_SUBPATH;
    not_inline_value!();
}

/// A group's `IterMap`, read-only, with reads living as long as the `GroupedMap` borrow.
pub type Group<'a, K, V> = At<'a, IterMap<K, V>>;

/// A group's `IterMap`, mutably borrowed from the `GroupedMap`.
pub struct GroupMut<'a, K, V> {
    map: IterMap<K, V>,
    _m: PhantomData<&'a mut ()>,
}
impl<K, V> core::ops::Deref for GroupMut<'_, K, V> {
    type Target = IterMap<K, V>;
    fn deref(&self) -> &IterMap<K, V> {
        &self.map
    }
}
impl<K, V> core::ops::DerefMut for GroupMut<'_, K, V> {
    fn deref_mut(&mut self) -> &mut IterMap<K, V> {
        &mut self.map
    }
}

impl<G: StorageKey + 'static, K: StorageKey + Clone + 'static, V: Value> GroupedMap<G, K, V> {
    #[track_caller]
    fn group_path(&self, g: &G) -> Path {
        bound(&self.path).kind(kind::ENTRIES).entry(g)
    }
    #[track_caller]
    pub fn group(&self, g: &G) -> Group<'_, K, V> {
        At::new(IterMap::__bound_at(self.group_path(g)))
    }
    #[track_caller]
    pub fn group_mut(&mut self, g: &G) -> GroupMut<'_, K, V> {
        GroupMut { map: IterMap::__bound_at(self.group_path(g)), _m: PhantomData }
    }
}

impl<'a, G: StorageKey + 'static, K: StorageKey + Clone + 'static, V: Value>
    At<'a, GroupedMap<G, K, V>>
{
    #[track_caller]
    pub fn group(&self, g: &G) -> Group<'a, K, V> {
        At::new(IterMap::__bound_at(self.inner.group_path(g)))
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
view_as_at!(Map<K, V> where K: StorageKey + 'static, V: Value);
view_as_at!(IterMap<K, V> where K: StorageKey + Clone + 'static, V: Value);
view_as_at!(Vec<T> where T: Value);
view_as_at!(Lazy<T> where T: Value);

impl<'a, R: Value, G: StorageKey + 'static, K: StorageKey + Clone + 'static, V: Value>
    ViewField<'a, R> for GroupedMap<G, K, V>
{
    type Out = At<'a, GroupedMap<G, K, V>>;
    fn __view_field(field_path: Path) -> Self::Out {
        At::new(GroupedMap { path: Binding::Bound(field_path), _m: PhantomData })
    }
}
impl<'a, R: Value, K: StorageKey + 'static> ViewField<'a, R> for Set<K> {
    type Out = At<'a, Set<K>>;
    fn __view_field(field_path: Path) -> Self::Out {
        At::new(Set(Map::__bound_at(field_path)))
    }
}
impl<'a, R: Value, K: StorageKey + Clone + 'static> ViewField<'a, R> for IterSet<K> {
    type Out = At<'a, IterSet<K>>;
    fn __view_field(field_path: Path) -> Self::Out {
        At::new(IterSet(IterMap::__bound_at(field_path)))
    }
}
