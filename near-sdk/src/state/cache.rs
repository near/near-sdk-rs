//! The one per-call cache, and the only code that can reach storage host functions.
//!
//! A read fills a slot, a write marks it dirty, a remove marks it deleted, and a mutable
//! hand-out marks it touched (re-encoded and compared at flush). The flush writes each
//! modified key once, in key order, with the root record last.
//!
//! # Safety invariant for the references collections hand out
//!
//! Collections return `&V` / `&mut V` that point into values owned by this cache. That is
//! sound under these rules, which every `unsafe` deref in `state` relies on:
//!
//! 1. Every slot and every decoded value is a separate heap allocation reached through a raw
//!    pointer. Map rebalancing never moves them, and the cache never forms a `&mut` over a
//!    value while a collection operation is not running.
//! 2. Within a call, a value is dropped or overwritten only by an operation that takes
//!    `&mut` on the collection handle that owns the key (`insert`, `remove`, `discard`,
//!    `clear`). Borrowck then guarantees no reference from *that handle* is alive.
//! 3. Exactly one handle can produce a `&mut` to a given key: the borrow chain from the
//!    single root. This is NOT type-enforced. Two roots alive at once (possible in tests by
//!    calling `state_read()` twice), or a `View` and a `&mut` record reached from different
//!    roots, can alias. See PROTOTYPE_NOTES.md.
//! 4. `begin_call` does not free the previous call's slots; it leaks them, so a reference that
//!    outlives a test's "call" stays valid (but stale). On chain the instance ends instead.

use super::key::Path;
use super::value::Value;
use core::any::Any;
use core::cell::RefCell;
use core::ptr::NonNull;
use std::collections::BTreeMap;

/// Every storage host call `state` makes goes through this trait. Private to the crate.
pub(crate) trait Host {
    fn read(&mut self, key: &[u8]) -> Option<Vec<u8>>;
    fn write(&mut self, key: &[u8], value: &[u8]);
    fn remove(&mut self, key: &[u8]);
}

/// The real host (or the mocked blockchain under `unit-testing`).
pub(crate) struct EnvHost;

impl Host for EnvHost {
    fn read(&mut self, key: &[u8]) -> Option<Vec<u8>> {
        crate::env::storage_read(key)
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
}

impl<V: Value> Erased for V {
    fn encode(&self, orig: Option<&[u8]>) -> Vec<u8> {
        self.encode_value(orig)
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
enum Orig {
    /// Never read in this call.
    Unknown,
    Absent,
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
}

impl Slot {
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
}

impl Cache {
    fn new(host: Box<dyn Host>) -> Self {
        Cache { host, slots: BTreeMap::new(), root: None }
    }

    fn slot(&mut self, key: &[u8]) -> NonNull<Slot> {
        if let Some(s) = self.slots.get(key) {
            return *s;
        }
        let s = NonNull::from(Box::leak(Box::new(Slot {
            orig: Orig::Unknown,
            value: None,
            mode: Mode::Clean,
        })));
        self.slots.insert(key.to_vec(), s);
        s
    }

    /// Ensures the slot's original bytes are known, reading the host at most once per call.
    fn load(&mut self, key: &[u8]) -> NonNull<Slot> {
        let mut sp = self.slot(key);
        // SAFETY: slots are only accessed inside cache operations (invariant 1).
        let s = unsafe { sp.as_mut() };
        if s.orig == Orig::Unknown && s.mode == Mode::Clean {
            s.orig = match self.host.read(key) {
                Some(b) => Orig::Present(b),
                None => Orig::Absent,
            };
        }
        sp
    }
}

std::thread_local! {
    static CACHE: RefCell<Cache> = RefCell::new(Cache::new(Box::new(EnvHost)));
}

fn with<R>(f: impl FnOnce(&mut Cache) -> R) -> R {
    CACHE.with(|c| f(&mut c.borrow_mut()))
}

/// Starts a new call: forgets every slot (leaking them, invariant 4), keeps the host.
pub(crate) fn begin_call() {
    with(|c| {
        c.slots = BTreeMap::new();
        c.root = None;
    });
}

/// Replaces the host and starts a new call.
pub(crate) fn install_host(host: Box<dyn Host>) {
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
    // Decode outside the RefCell borrow: decoding a record binds its collections, which
    // never touches the cache, but keep the borrow short regardless.
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

/// Writes a value without reading the key first.
pub(crate) fn put<V: Value>(path: &Path, v: V) -> NonNull<V> {
    let mut sp = with(|c| c.slot(path.as_bytes()));
    // SAFETY: invariants 1 and 2.
    let s = unsafe { sp.as_mut() };
    s.drop_value();
    let b: Box<dyn Erased> = Box::new(v);
    let p = NonNull::from(Box::leak(b));
    s.value = Some(p);
    s.mode = Mode::Dirty;
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

/// Flushes every modified key once in key order, then the root record last.
///
/// `root` is `(root path, encoder)`; the encoder receives the root's original bytes so it can
/// preserve unknown tags. Returns whether the root was written.
pub(crate) fn flush(root: Option<(&Path, &dyn Fn(Option<&[u8]>) -> Vec<u8>)>) -> bool {
    with(|c| {
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
                    let p = s.value.expect("dirty slot has a value");
                    // SAFETY: invariant 1.
                    let enc = unsafe { p.as_ref() }.encode(s.orig.bytes());
                    if s.orig.bytes() != Some(&enc[..]) {
                        c.host.write(&key, &enc);
                    }
                    s.orig = Orig::Present(enc);
                    s.mode = Mode::Clean;
                }
                Mode::Deleted => {
                    if s.orig != Orig::Absent {
                        c.host.remove(&key);
                    }
                    s.orig = Orig::Absent;
                    s.mode = Mode::Clean;
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
            s.orig = Orig::Present(enc);
        }
        wrote_root
    })
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
                    let p = s.value.expect("dirty slot has a value");
                    // SAFETY: invariant 1.
                    let enc = unsafe { p.as_ref() }.encode(s.orig.bytes());
                    s.orig.bytes() != Some(&enc[..])
                }
            }
        })
    })
}
