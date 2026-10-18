//! The one per-call cache, and the only code that can reach storage host functions.
//!
//! A read fills a slot, a write marks it dirty, a remove marks it deleted, and a mutable
//! hand-out marks it touched (re-encoded and compared at flush). The flush writes each
//! modified key once, in key order, with the root record last, and then drops every slot.
//!
//! # Safety invariant for the references collections hand out
//!
//! Collections return `&V` / `&mut V` that point into values owned by this cache. That is
//! sound under these rules, which every `unsafe` deref in `state` relies on:
//!
//! 1. Every slot and every decoded value is a separate heap allocation reached through a raw
//!    pointer. LookupMap rebalancing never moves them, and the cache never forms a `&mut` over a
//!    value while a collection operation is not running.
//! 2. Within a call, a value is dropped or overwritten only by an operation that takes
//!    `&mut` on the collection handle that owns the key (`insert`, `remove`, `discard`,
//!    `clear`). Borrowck then guarantees no reference from *that handle* is alive.
//! 3. Exactly one handle can produce a `&mut` to a given key: the borrow chain from the
//!    single root. This is NOT type-enforced. Two roots alive at once (possible in tests by
//!    calling `state_read()` twice) can alias.
//! 4. The flush (and `begin_call`) frees every slot. No reference obtained from the root may
//!    be alive at the flush. The wrapper guarantees this (it flushes after the method body
//!    returns); code calling `state_write(&root)` by hand while holding a reference does not,
//!    and borrowck cannot catch it because both borrows are shared. Freeing at the flush is
//!    what makes a v1 collection nested in a v2 record run its write-on-drop.

use super::key::Path;
use super::value::Value;
use core::any::Any;
use core::cell::RefCell;
use core::ptr::NonNull;
use std::collections::BTreeMap;

/// Every storage host call `state` makes goes through this trait. Private to the crate.
pub(crate) trait Host {
    fn read(&mut self, key: &[u8]) -> Option<Vec<u8>>;
    fn has_key(&mut self, key: &[u8]) -> bool;
    fn write(&mut self, key: &[u8], value: &[u8]);
    fn remove(&mut self, key: &[u8]);
}

/// The real host (or the mocked blockchain under `unit-testing`).
pub(crate) struct EnvHost;

impl Host for EnvHost {
    fn read(&mut self, key: &[u8]) -> Option<Vec<u8>> {
        crate::env::storage_read(key)
    }
    fn has_key(&mut self, key: &[u8]) -> bool {
        crate::env::storage_has_key(key)
    }
    fn write(&mut self, key: &[u8], value: &[u8]) {
        crate::env::storage_write(key, value);
    }
    fn remove(&mut self, key: &[u8]) {
        crate::env::storage_remove(key);
    }
}

trait Erased: Any {
    fn encode(&self, orig: Option<&[u8]>) -> Vec<u8>;
    fn as_any_mut(&mut self) -> &mut dyn Any;
    /// Whether the value owns storage below its key (so it may carry the clearing flag).
    fn owns_storage(&self) -> bool;
}

impl<V: Value> Erased for V {
    fn encode(&self, orig: Option<&[u8]>) -> Vec<u8> {
        self.encode_value(orig)
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
    fn owns_storage(&self) -> bool {
        V::MAX_SUBPATH > 0
    }
}

/// D9 clearing state of a record slot.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Clearing {
    /// Whatever the stored bytes say (flag present or not).
    FromOrig,
    /// `remove_some` started or continued clearing in this call: the flush writes the flag.
    Set,
    /// Replaced or removed in this call: not clearing, whatever the stored bytes said.
    Cleared,
}

#[derive(Clone, PartialEq, Eq, Debug)]
enum Orig {
    /// Never read in this call.
    Unknown,
    Absent,
    /// Known present (from `storage_has_key`), bytes not read.
    PresentUnread,
    Present(Vec<u8>),
}

impl Orig {
    fn bytes(&self) -> Option<&[u8]> {
        match self {
            Orig::Present(b) => Some(b),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Mode {
    Clean,
    /// Handed out as `&mut`; written at flush only if its encoding changed.
    Touched,
    /// Explicitly written.
    Dirty,
    /// Explicitly removed.
    Deleted,
}

struct Slot {
    orig: Orig,
    /// Current decoded value; `None` means absent (or never decoded, for raw slots).
    value: Option<NonNull<dyn Erased>>,
    mode: Mode,
    /// Whether the flush merges unknown tags from `orig`. A blind `put` replaces the value
    /// whole, so it clears this; read-modify-write keeps it.
    merge: bool,
    clearing: Clearing,
}

impl Slot {
    fn is_clearing(&self) -> bool {
        match self.clearing {
            Clearing::Set => true,
            Clearing::Cleared => false,
            Clearing::FromOrig => self.orig.bytes().is_some_and(super::record::is_clearing),
        }
    }

    /// The bytes the flush would write for a Touched/Dirty slot. A record being cleared gets
    /// the flag prepended (the record encoder never merges it from `orig`).
    fn encode(&self) -> Vec<u8> {
        let p = self.value.expect("dirty slot has a value");
        let merge_from = if self.merge { self.orig.bytes() } else { None };
        // SAFETY: invariant 1.
        let v = unsafe { p.as_ref() };
        let enc = v.encode(merge_from);
        if v.owns_storage() && self.is_clearing() {
            let mut out = super::record::CLEARING_FLAG.to_vec();
            out.extend_from_slice(&enc);
            out
        } else {
            enc
        }
    }

    fn drop_value(&mut self) {
        if let Some(p) = self.value.take() {
            // SAFETY: invariant 2; the caller holds `&mut` on the owning handle.
            unsafe { drop(Box::from_raw(p.as_ptr())) };
        }
    }
}

pub(crate) struct Cache {
    host: Box<dyn Host>,
    slots: BTreeMap<Vec<u8>, NonNull<Slot>>,
    /// Key of the root record, excluded from the ordered pass and written last.
    root: Option<Vec<u8>>,
    /// Collection handles handed out by reference (`group_mut`); freed with the slots.
    handles: std::vec::Vec<NonNull<dyn Any>>,
}

impl Cache {
    fn new(host: Box<dyn Host>) -> Self {
        Cache { host, slots: BTreeMap::new(), root: None, handles: std::vec::Vec::new() }
    }

    fn slot(&mut self, key: &[u8]) -> NonNull<Slot> {
        if let Some(s) = self.slots.get(key) {
            return *s;
        }
        let s = NonNull::from(Box::leak(Box::new(Slot {
            orig: Orig::Unknown,
            value: None,
            mode: Mode::Clean,
            merge: true,
            clearing: Clearing::FromOrig,
        })));
        self.slots.insert(key.to_vec(), s);
        s
    }

    /// Ensures the slot's original bytes are known, reading the host at most once per call.
    fn load(&mut self, key: &[u8]) -> NonNull<Slot> {
        let mut sp = self.slot(key);
        // SAFETY: slots are only accessed inside cache operations (invariant 1).
        let s = unsafe { sp.as_mut() };
        let unread = matches!(s.orig, Orig::Unknown | Orig::PresentUnread);
        if unread && s.value.is_none() && s.mode == Mode::Clean {
            s.orig = match self.host.read(key) {
                Some(b) => Orig::Present(b),
                None => Orig::Absent,
            };
        }
        sp
    }

    /// Takes every slot and handle out of the cache; the caller drops them outside the
    /// `RefCell` borrow.
    fn take_slots(&mut self) -> Taken {
        self.root = None;
        Taken {
            slots: core::mem::take(&mut self.slots).into_values().collect(),
            handles: core::mem::take(&mut self.handles),
        }
    }
}

struct Taken {
    slots: std::vec::Vec<NonNull<Slot>>,
    handles: std::vec::Vec<NonNull<dyn Any>>,
}

/// Frees slots and their values, then handles (outside the cache borrow, so a value's `Drop`,
/// e.g. a v1 collection's write-on-drop, may call the host).
fn free_slots(taken: Taken) {
    for sp in taken.slots {
        // SAFETY: invariant 4; the slot has been removed from the cache.
        let mut slot = unsafe { Box::from_raw(sp.as_ptr()) };
        slot.drop_value();
    }
    for hp in taken.handles {
        // SAFETY: invariant 4 (handles are freed with the slots).
        unsafe { drop(Box::from_raw(hp.as_ptr())) };
    }
}

/// Moves a collection handle into the per-call arena and returns a pointer that stays valid
/// until the next flush or `begin_call` (invariant 4). Used so `GroupedMap::group_mut` can hand
/// out `&'a mut Map` whose reads live as long as the `GroupedMap` borrow, not a temporary.
pub(crate) fn arena<T: Any>(v: T) -> NonNull<T> {
    let p = NonNull::from(Box::leak(Box::new(v)));
    with(|c| c.handles.push(p));
    p
}

std::thread_local! {
    static CACHE: RefCell<Cache> = RefCell::new(Cache::new(Box::new(EnvHost)));
}

fn with<R>(f: impl FnOnce(&mut Cache) -> R) -> R {
    CACHE.with(|c| f(&mut c.borrow_mut()))
}

/// Starts a new call: frees every slot, keeps the host.
pub(crate) fn begin_call() {
    free_slots(with(|c| c.take_slots()));
}

/// Replaces the host and starts a new call.
pub(crate) fn install_host(host: Box<dyn Host>) {
    begin_call();
    with(|c| *c = Cache::new(host));
}

/// Number of live slots, for the cache-memory notes.
pub(crate) fn slot_count() -> usize {
    with(|c| c.slots.len())
}

/// Reads and decodes the value at `path`, once per call. `None` if absent or deleted.
#[track_caller]
pub(crate) fn get<V: Value>(path: &Path) -> Option<NonNull<V>> {
    let key = path.as_bytes();
    let (mut sp, bytes) = with(|c| {
        let mut sp = c.load(key);
        // SAFETY: invariant 1.
        let s = unsafe { sp.as_mut() };
        if s.value.is_some() || s.mode == Mode::Deleted {
            (sp, None)
        } else {
            (sp, s.orig.bytes().map(<[u8]>::to_vec))
        }
    });
    // SAFETY: invariant 1.
    let s = unsafe { sp.as_mut() };
    if let Some(bytes) = bytes {
        let v = V::decode_value(&bytes, path).unwrap_or_else(|e| {
            crate::env::panic_str(&format!("state: cannot decode value at {path:?}: {e}"))
        });
        let b: Box<dyn Erased> = Box::new(v);
        s.value = Some(NonNull::from(Box::leak(b)));
    }
    let p = s.value?;
    // SAFETY: invariant 1; a value is only ever reached through its own slot.
    let any = unsafe { (*p.as_ptr()).as_any_mut() };
    match any.downcast_mut::<V>() {
        Some(v) => Some(NonNull::from(v)),
        None => panic!(
            "state: key {path:?} accessed as two different types: {}",
            core::any::type_name::<V>()
        ),
    }
}

/// Whether a value exists at `path`: answered from the cache, else one `storage_has_key`
/// (no value bytes moved).
pub(crate) fn exists(path: &Path) -> bool {
    with(|c| {
        let key = path.as_bytes();
        let mut sp = c.slot(key);
        // SAFETY: invariant 1.
        let s = unsafe { sp.as_mut() };
        if s.value.is_some() {
            return true;
        }
        if s.mode == Mode::Deleted {
            return false;
        }
        match s.orig {
            Orig::Absent => false,
            Orig::Present(_) | Orig::PresentUnread => true,
            Orig::Unknown => {
                let present = c.host.has_key(key);
                s.orig = if present { Orig::PresentUnread } else { Orig::Absent };
                present
            }
        }
    })
}

/// D9, for `at_mut`: `None` if no record is stored at `path`, else whether it is being
/// cleared. Reads the record's bytes (1 read, no decode) unless this call already has them:
/// the flag lives in the record, so `storage_has_key` cannot see it.
pub(crate) fn record_state(path: &Path) -> Option<bool> {
    with(|c| {
        let sp = c.load(path.as_bytes());
        // SAFETY: invariant 1.
        let s = unsafe { sp.as_ref() };
        if s.mode == Mode::Deleted {
            return None;
        }
        if s.value.is_none() && s.orig.bytes().is_none() {
            return None;
        }
        Some(s.is_clearing())
    })
}

/// D9: whether the (already loaded) record at `path` is being cleared.
pub(crate) fn is_clearing(path: &Path) -> bool {
    with(|c| {
        c.slots.get(path.as_bytes()).is_some_and(|s| {
            // SAFETY: invariant 1.
            unsafe { s.as_ref() }.is_clearing()
        })
    })
}

/// D9: marks the (decoded) record at `path` as being cleared; the flush writes the flag.
pub(crate) fn set_clearing(path: &Path) {
    with(|c| {
        let mut sp = c.slot(path.as_bytes());
        // SAFETY: invariant 1.
        let s = unsafe { sp.as_mut() };
        debug_assert!(s.value.is_some(), "set_clearing on a record that was not read");
        s.clearing = Clearing::Set;
        if s.mode == Mode::Clean {
            s.mode = Mode::Touched;
        }
    })
}

/// Marks a key whose value was handed out as `&mut`.
pub(crate) fn touch(path: &Path) {
    with(|c| {
        let mut sp = c.slot(path.as_bytes());
        // SAFETY: invariant 1.
        let s = unsafe { sp.as_mut() };
        if s.mode == Mode::Clean {
            s.mode = Mode::Touched;
        }
    })
}

/// Writes a value without reading the key first. The value replaces whatever is stored,
/// unknown tags included.
pub(crate) fn put<V: Value>(path: &Path, v: V) -> NonNull<V> {
    let mut sp = with(|c| c.slot(path.as_bytes()));
    // SAFETY: invariants 1 and 2.
    let s = unsafe { sp.as_mut() };
    s.drop_value();
    let b: Box<dyn Erased> = Box::new(v);
    let p = NonNull::from(Box::leak(b));
    s.value = Some(p);
    s.mode = Mode::Dirty;
    s.merge = false;
    s.clearing = Clearing::Cleared;
    // SAFETY: just allocated as a `V`.
    let any = unsafe { (*p.as_ptr()).as_any_mut() };
    NonNull::from(any.downcast_mut::<V>().expect("just stored"))
}

/// Removes the value at `path`, returning it if it was decoded. No host read.
pub(crate) fn delete<V: Value>(path: &Path) -> Option<V> {
    let mut sp = with(|c| c.slot(path.as_bytes()));
    // SAFETY: invariants 1 and 2.
    let s = unsafe { sp.as_mut() };
    s.mode = Mode::Deleted;
    s.clearing = Clearing::Cleared;
    let p = s.value.take()?;
    // SAFETY: invariant 2; we own the value now.
    let b = unsafe { Box::from_raw(p.as_ptr()) };
    let any: Box<dyn Any> = b;
    any.downcast::<V>().ok().map(|b| *b)
}

/// Reads the raw root bytes and remembers the root key so the flush writes it last.
pub(crate) fn read_root(path: &Path) -> Option<Vec<u8>> {
    with(|c| {
        c.root = Some(path.as_bytes().to_vec());
        let sp = c.load(path.as_bytes());
        // SAFETY: invariant 1.
        unsafe { sp.as_ref() }.orig.bytes().map(<[u8]>::to_vec)
    })
}

/// Original bytes stored at the root key, if read in this call.
pub(crate) fn root_orig(path: &Path) -> Option<Vec<u8>> {
    with(|c| {
        c.slots
            .get(path.as_bytes())
            // SAFETY: invariant 1.
            .and_then(|s| unsafe { s.as_ref() }.orig.bytes().map(<[u8]>::to_vec))
    })
}

/// Flushes every modified key once in key order, then the root record last, then frees
/// every slot (invariant 4).
///
/// `root` is `(root path, encoder)`; the encoder receives the root's original bytes so it can
/// preserve unknown tags. Returns whether the root was written.
pub(crate) fn flush(root: Option<(&Path, &dyn Fn(Option<&[u8]>) -> Vec<u8>)>) -> bool {
    let (wrote_root, slots) = with(|c| {
        let root_key = root.map(|(p, _)| p.as_bytes().to_vec()).or_else(|| c.root.clone());
        let keys: std::vec::Vec<(std::vec::Vec<u8>, NonNull<Slot>)> =
            c.slots.iter().map(|(k, s)| (k.clone(), *s)).collect();
        for (key, mut sp) in keys {
            if Some(&key) == root_key.as_ref() {
                continue;
            }
            // SAFETY: flush runs after the method body; invariant 1.
            let s = unsafe { sp.as_mut() };
            match s.mode {
                Mode::Clean => {}
                Mode::Touched | Mode::Dirty => {
                    let enc = s.encode();
                    if s.orig.bytes() != Some(&enc[..]) {
                        c.host.write(&key, &enc);
                    }
                }
                Mode::Deleted => {
                    if s.orig != Orig::Absent {
                        c.host.remove(&key);
                    }
                }
            }
        }
        let mut wrote_root = false;
        if let Some((path, encode)) = root {
            let mut sp = c.slot(path.as_bytes());
            // SAFETY: invariant 1.
            let s = unsafe { sp.as_mut() };
            let enc = encode(s.orig.bytes());
            if s.orig.bytes() != Some(&enc[..]) {
                c.host.write(path.as_bytes(), &enc);
                wrote_root = true;
            }
        }
        (wrote_root, c.take_slots())
    });
    free_slots(slots);
    wrote_root
}

/// Whether any key would be written or removed by a flush (the root excluded).
pub(crate) fn has_pending_writes() -> bool {
    with(|c| {
        c.slots.iter().any(|(k, s)| {
            if Some(k) == c.root.as_ref() {
                return false;
            }
            // SAFETY: invariant 1.
            let s = unsafe { s.as_ref() };
            match s.mode {
                Mode::Clean => false,
                Mode::Deleted => s.orig != Orig::Absent,
                Mode::Touched | Mode::Dirty => {
                    let enc = s.encode();
                    s.orig.bytes() != Some(&enc[..])
                }
            }
        })
    })
}

/// Whether this call has changes the flush has not written yet (any slot written, removed or
/// handed out as `&mut`). The D5 `storage_usage` guard keys on this. A change to a root scalar
/// alone is not seen (the root is encoded only at the flush).
pub(crate) fn has_unflushed_changes() -> bool {
    with(|c| {
        c.slots.values().any(|s| {
            // SAFETY: invariant 1.
            unsafe { s.as_ref() }.mode != Mode::Clean
        })
    })
}

/// `storage_has_key` through the host, for `ContractState::state_exists` on a v2 root.
pub(crate) fn host_has_key(key: &[u8]) -> bool {
    with(|c| c.host.has_key(key))
}
