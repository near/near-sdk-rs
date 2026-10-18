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
//!    value while a collection operation is not running. The read path (`get`) forms no `&mut`
//!    at all: it type-checks through a shared borrow and returns a pointer cast, so a `&V`
//!    handed out earlier stays valid under Stacked and Tree Borrows (audit A1).
//! 2. Within a call, a value is replaced or removed only by an operation that takes `&mut` on
//!    the collection handle that owns the key (`insert`, `remove`, `discard`, `clear`).
//!    Borrowck then guarantees no reference from *that handle* is alive. A second handle to
//!    the same path (a field moved out with `mem::take`, audit A2) is not seen by borrowck, so
//!    a value that is overwritten (`put`) or deleted without being returned (`delete`) is not
//!    freed: it moves to the call's graveyard and is freed with the slots at the flush, and a
//!    `&V` from the other handle stays valid for the call. A removal that *returns* the value
//!    (`take`: `remove`, `replace`, `pop`, `swap_remove`) has to move it out, so a `&V` to it
//!    from an escaped second handle still dangles; that one hole is documented, not closed.
//! 3. Exactly one handle can produce a `&mut` to a given key: the borrow chain from the
//!    single root. This is NOT type-enforced. Two roots alive at once (possible in tests by
//!    calling `state_read()` twice) can alias.
//! 4. The flush (and `begin_call`) frees every slot and the graveyard. No reference obtained from the root may
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
    /// A shared view for the type check. Never `&mut`: a `&mut` retag of a value on the read
    /// path would invalidate every `&V` already handed out for it (audit A1, Stacked Borrows).
    fn as_any(&self) -> &dyn Any;
}

impl<V: Value> Erased for V {
    fn encode(&self, orig: Option<&[u8]>) -> Vec<u8> {
        self.encode_value(orig)
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// D9 clearing state of a record slot.
///
/// A record that `remove_step` is clearing across calls is stored at its FLAG KEY
/// ([`Path::flag`], the record path plus one `00` byte) and its own key is absent. So the
/// record key's existence alone says "present and not clearing", and `at_mut` stays one
/// `storage_has_key`; only a miss at the record key consults the flag key.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Clearing {
    /// Whatever storage says: clearing iff the bytes were found at the flag key.
    FromOrig,
    /// `remove_step` started or continued clearing in this call: the flush stores the record
    /// at the flag key (and removes the record key).
    Set,
    /// Replaced or removed in this call: not clearing, whatever storage said.
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
    /// A map entry whose value owns storage, so it may be stored at its flag key (D9). Only
    /// such slots consult the flag key, and only after a miss at the record key.
    flaggable: bool,
    /// `orig` describes the flag key, not the record key (which is then known absent).
    at_flag: bool,
    /// A blind delete (no read, `orig` unknown) of a flaggable entry happened in this call, so
    /// the record it removed may have been at either key. Kept through a later `put`: the
    /// flush then removes the key the new value is not written to (review F1).
    blind_delete: bool,
    /// `set_clearing` found the record unmodified in this call: the flush moves its stored
    /// bytes to the flag key verbatim instead of re-encoding it, which could materialise
    /// `default = ..` fields and change its size (review F3).
    verbatim: bool,
}

/// What a flush does for one slot: the writes and removes, by key.
enum FlushOp {
    Write(Vec<u8>, Vec<u8>),
    Remove(Vec<u8>),
}

impl Slot {
    fn is_clearing(&self) -> bool {
        match self.clearing {
            Clearing::Set => true,
            Clearing::Cleared => false,
            Clearing::FromOrig => self.at_flag && self.orig != Orig::Absent,
        }
    }

    /// The bytes the flush would write for a Touched/Dirty slot.
    fn encode(&self) -> Vec<u8> {
        let p = self.value.expect("dirty slot has a value");
        let merge_from = if self.merge { self.orig.bytes() } else { None };
        // SAFETY: invariant 1.
        unsafe { p.as_ref() }.encode(merge_from)
    }

    /// The host operations a flush performs for this slot at `key`. A record being cleared
    /// goes to its flag key and leaves its record key; a record no longer being cleared goes
    /// back to its record key and leaves the flag key. Unchanged bytes at the same key are
    /// not rewritten.
    fn flush_ops(&self, key: &[u8]) -> std::vec::Vec<FlushOp> {
        let mut ops = std::vec::Vec::new();
        let present = matches!(self.orig, Orig::Present(_) | Orig::PresentUnread);
        let (mut at_key, mut at_flag) = (present && !self.at_flag, present && self.at_flag);
        if self.blind_delete && self.orig == Orig::Unknown {
            // F1: a blind delete then a reinsert: either key may still hold the old record.
            (at_key, at_flag) = (true, true);
        }
        match self.mode {
            Mode::Clean => {}
            Mode::Touched | Mode::Dirty => {
                let enc = match self.orig.bytes() {
                    Some(b) if self.verbatim && self.is_clearing() => b.to_vec(),
                    _ => self.encode(),
                };
                let same = self.orig.bytes() == Some(&enc[..]);
                if self.is_clearing() {
                    // Key order (review F4): the record key sorts before its flag key.
                    if at_key {
                        ops.push(FlushOp::Remove(key.to_vec()));
                    }
                    if !(at_flag && same) {
                        ops.push(FlushOp::Write(flag_key(key), enc));
                    }
                } else {
                    if !(at_key && same) {
                        ops.push(FlushOp::Write(key.to_vec(), enc));
                    }
                    if at_flag {
                        ops.push(FlushOp::Remove(flag_key(key)));
                    }
                }
            }
            Mode::Deleted => match self.orig {
                Orig::Absent => {}
                // A blind delete (`discard`, a bulk clear): the record may be at either key.
                Orig::Unknown => {
                    ops.push(FlushOp::Remove(key.to_vec()));
                    if self.flaggable {
                        ops.push(FlushOp::Remove(flag_key(key)));
                    }
                }
                _ if self.at_flag => ops.push(FlushOp::Remove(flag_key(key))),
                _ => ops.push(FlushOp::Remove(key.to_vec())),
            },
        }
        ops
    }

    fn drop_value(&mut self) {
        if let Some(p) = self.value.take() {
            // SAFETY: invariant 4; only the flush frees a slot, with no reference alive.
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
    /// Values overwritten or deleted in this call, freed with the slots (invariant 2).
    graveyard: std::vec::Vec<NonNull<dyn Erased>>,
    /// This call's removals of values that own storage, and `remove_step` clearings:
    /// `(epoch, key)`, in epoch order. A handle bound below `key` before `epoch` is stale
    /// (audit A2); see [`check_live`].
    gone: std::vec::Vec<(u64, Vec<u8>)>,
}

impl Cache {
    fn new(host: Box<dyn Host>) -> Self {
        Cache {
            host,
            slots: BTreeMap::new(),
            root: None,
            handles: std::vec::Vec::new(),
            graveyard: std::vec::Vec::new(),
            gone: std::vec::Vec::new(),
        }
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
            flaggable: false,
            at_flag: false,
            blind_delete: false,
            verbatim: false,
        })));
        self.slots.insert(key.to_vec(), s);
        s
    }

    /// The slot for `key`, marked as a map entry that owns storage when `flaggable`.
    fn entry_slot(&mut self, key: &[u8], flaggable: bool) -> NonNull<Slot> {
        let mut sp = self.slot(key);
        if flaggable {
            // SAFETY: invariant 1.
            unsafe { sp.as_mut() }.flaggable = true;
        }
        sp
    }

    /// Ensures the slot's original bytes are known, reading the host at most once per call.
    /// A flaggable slot that misses at its key reads its flag key (D9).
    fn load(&mut self, key: &[u8]) -> NonNull<Slot> {
        let mut sp = self.slot(key);
        // SAFETY: slots are only accessed inside cache operations (invariant 1).
        let s = unsafe { sp.as_mut() };
        let unread = matches!(s.orig, Orig::Unknown | Orig::PresentUnread);
        if unread && s.value.is_none() && s.mode == Mode::Clean {
            let at = if s.at_flag { flag_key(key) } else { key.to_vec() };
            s.orig = match self.host.read(&at) {
                Some(b) => Orig::Present(b),
                None if s.flaggable && !s.at_flag => match self.host.read(&flag_key(key)) {
                    Some(b) => {
                        s.at_flag = true;
                        Orig::Present(b)
                    }
                    None => Orig::Absent,
                },
                None => Orig::Absent,
            };
        }
        sp
    }

    /// Takes every slot and handle out of the cache; the caller drops them outside the
    /// `RefCell` borrow.
    fn take_slots(&mut self) -> Taken {
        self.root = None;
        self.gone.clear();
        Taken {
            slots: core::mem::take(&mut self.slots).into_values().collect(),
            handles: core::mem::take(&mut self.handles),
            graveyard: core::mem::take(&mut self.graveyard),
        }
    }

    /// Logs that the value at `key`, which owns storage, was removed or started clearing: every
    /// handle bound below it before now is stale (audit A2).
    fn record_gone(&mut self, key: &[u8]) {
        let e = EPOCH.with(|c| {
            let e = c.get() + 1;
            c.set(e);
            e
        });
        self.gone.push((e, key.to_vec()));
    }
}

struct Taken {
    slots: std::vec::Vec<NonNull<Slot>>,
    handles: std::vec::Vec<NonNull<dyn Any>>,
    graveyard: std::vec::Vec<NonNull<dyn Erased>>,
}

/// Frees slots and their values, then handles (outside the cache borrow, so a value's `Drop`,
/// e.g. a v1 collection's write-on-drop, may call the host).
fn free_slots(taken: Taken) {
    for sp in taken.slots {
        // SAFETY: invariant 4; the slot has been removed from the cache.
        let mut slot = unsafe { Box::from_raw(sp.as_ptr()) };
        slot.drop_value();
    }
    for vp in taken.graveyard {
        // SAFETY: invariant 4; buried values are owned by the graveyard alone.
        unsafe { drop(Box::from_raw(vp.as_ptr())) };
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
    /// Removal epoch (audit A2): bumped by every removal of a value that owns storage and every
    /// `remove_step` clearing. Never reset, not even by `install_host`, so a handle's stamp from
    /// an earlier call is always older than this call's removals.
    static EPOCH: core::cell::Cell<u64> = const { core::cell::Cell::new(0) };
}

/// The current removal epoch; a handle is stamped with it when bound.
pub(crate) fn epoch() -> u64 {
    EPOCH.with(core::cell::Cell::get)
}

/// Audit A2: panics if a value that owns storage was removed (or started `remove_step`
/// clearing) at a key strictly below which `path` lies, after the handle at `path` was
/// stamped with `seen`. Such a handle escaped its record (`mem::take` on a field), and writing
/// through it would orphan keys under a removed record or bypass the clearing guard. A handle
/// bound after the removal (a record re-inserted at the same key) is fresh and passes.
///
/// Cost: one thread-local load and one `u64` compare while no removal happened since the
/// handle last checked; otherwise a scan of only the removals newer than `seen`, then `seen`
/// moves to now, so each removal is scanned at most once per handle.
#[track_caller]
pub(crate) fn check_live(path: &Path, seen: &mut u64) {
    let now = epoch();
    if *seen == now {
        return;
    }
    let key = path.as_bytes();
    let stale = with(|c| {
        c.gone
            .iter()
            .rev()
            .take_while(|(e, _)| *e > *seen)
            .any(|(_, gone)| key.len() > gone.len() && key.starts_with(gone))
    });
    if stale {
        panic!(
            "state: the collection at {path:?} belongs to a record that was removed or started \
             `remove_step` clearing in this call after this handle was made; a handle moved out \
             of its record (`mem::take`/`mem::replace` on a field) cannot write any more"
        );
    }
    *seen = now;
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

/// The flag key of the record whose key is `key`: one more byte, `00` (see [`Path::flag`]),
/// length-checked like every path hop.
#[track_caller]
fn flag_key(key: &[u8]) -> Vec<u8> {
    super::key::flag_key_bytes(key)
}

/// Reads and decodes the value at `path`, once per call. `None` if absent or deleted.
#[track_caller]
pub(crate) fn get<V: Value>(path: &Path) -> Option<NonNull<V>> {
    get_impl::<V>(path, false)
}

/// [`get`] for a map entry. If `V` owns storage, a miss at the record key reads the flag key,
/// where a record being cleared lives (D9): 1 more read on a miss, none on a hit.
#[track_caller]
pub(crate) fn get_entry<V: Value>(path: &Path) -> Option<NonNull<V>> {
    get_impl::<V>(path, V::MAX_SUBPATH > 0)
}

#[track_caller]
fn get_impl<V: Value>(path: &Path, flaggable: bool) -> Option<NonNull<V>> {
    let key = path.as_bytes();
    let (mut sp, bytes) = with(|c| {
        c.entry_slot(key, flaggable);
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
    Some(typed::<V>(p, path))
}

/// `p` as a `NonNull<V>`, after checking through a shared borrow that it holds a `V`. The
/// pointer keeps the provenance of the box (no `&mut` is formed), so a read never invalidates
/// a `&V` handed out earlier for the same value (audit A1).
#[track_caller]
fn typed<V: Value>(p: NonNull<dyn Erased>, path: &Path) -> NonNull<V> {
    // SAFETY: invariant 1; a shared borrow of a live value, dropped before returning.
    let is_v = unsafe { p.as_ref() }.as_any().is::<V>();
    if !is_v {
        panic!("state: key {path:?} accessed as two different types: {}", core::any::type_name::<V>());
    }
    p.cast::<V>()
}

/// Whether a value exists at `path`: answered from the cache, else one `storage_has_key`
/// (no value bytes moved).
pub(crate) fn exists(path: &Path) -> bool {
    exists_impl(path, false)
}

/// [`exists`] for a map entry. If `V` owns storage, a miss at the record key checks the flag
/// key (D9): 1 more `storage_has_key` on a miss, none on a hit. A record being cleared exists.
pub(crate) fn exists_entry<V: Value>(path: &Path) -> bool {
    exists_impl(path, V::MAX_SUBPATH > 0)
}

fn exists_impl(path: &Path, flaggable: bool) -> bool {
    with(|c| {
        let key = path.as_bytes();
        let mut sp = c.entry_slot(key, flaggable);
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
                let mut present = c.host.has_key(key);
                if !present && s.flaggable {
                    present = c.host.has_key(&flag_key(key));
                    s.at_flag = present;
                }
                s.orig = if present { Orig::PresentUnread } else { Orig::Absent };
                present
            }
        }
    })
}

/// D9, for `at_mut`: `None` if no record is stored at `path`, else whether it is being
/// cleared. 1 `storage_has_key` on the record key unless cached; the record is not read. Only
/// a miss checks the flag key (a 2nd `storage_has_key`), since a record being cleared is
/// stored there and its record key is absent.
pub(crate) fn record_state<V: Value>(path: &Path) -> Option<bool> {
    if !exists_entry::<V>(path) {
        return None;
    }
    Some(is_clearing(path))
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

/// D9: marks the (decoded) record at `path` as being cleared; the flush moves it to its flag
/// key.
pub(crate) fn set_clearing(path: &Path) {
    with(|c| {
        let mut sp = c.slot(path.as_bytes());
        // SAFETY: invariant 1.
        let s = unsafe { sp.as_mut() };
        debug_assert!(s.value.is_some(), "set_clearing on a record that was not read");
        s.clearing = Clearing::Set;
        if s.mode == Mode::Clean {
            s.mode = Mode::Touched;
            s.verbatim = true;
        }
        c.record_gone(path.as_bytes());
    })
}

/// The bytes stored at `path` before this call, for the removal check (API round 5, A3):
/// 1 read unless this call already read the key (a flaggable entry that misses reads its flag
/// key too, as `get_entry`). `None` if nothing is stored, or if this call already replaced the
/// value whole or deleted it (the old bytes are then no longer what the removal removes).
pub(crate) fn stored_bytes(path: &Path, flaggable: bool) -> Option<Vec<u8>> {
    with(|c| {
        let key = path.as_bytes();
        c.entry_slot(key, flaggable);
        let sp = c.load(key);
        // SAFETY: invariant 1.
        let s = unsafe { sp.as_ref() };
        match s.mode {
            Mode::Deleted => None,
            Mode::Dirty if !s.merge => None,
            _ => s.orig.bytes().map(<[u8]>::to_vec),
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
    let mut sp = with(|c| {
        let mut sp = c.slot(path.as_bytes());
        // SAFETY: invariants 1 and 2. The old value is buried, not freed: an escaped second
        // handle may still hold a `&V` to it (audit A2).
        if let Some(old) = unsafe { sp.as_mut() }.value.take() {
            c.graveyard.push(old);
        }
        sp
    });
    // SAFETY: invariants 1 and 2.
    let s = unsafe { sp.as_mut() };
    let b: Box<dyn Erased> = Box::new(v);
    let p = NonNull::from(Box::leak(b));
    s.value = Some(p);
    s.mode = Mode::Dirty;
    s.merge = false;
    s.clearing = Clearing::Cleared;
    s.verbatim = false;
    // Just allocated as a `V`; `cast` keeps the box's provenance.
    p.cast::<V>()
}

/// Removes the value at `path`, returning it if it was decoded. No host read. The value moves
/// out of the cache, so this is only for removals that hand it back (invariant 2).
pub(crate) fn take<V: Value>(path: &Path) -> Option<V> {
    let p = delete_impl::<V>(path, false)?;
    // SAFETY: invariant 2; the slot no longer points at the value, so we own it now.
    let b = unsafe { Box::from_raw(p.as_ptr()) };
    let any: Box<dyn Any> = b;
    any.downcast::<V>().ok().map(|b| *b)
}

/// [`take`] for a map entry; see [`delete_entry`] for the flag key.
pub(crate) fn take_entry<V: Value>(path: &Path) -> Option<V> {
    let p = delete_impl::<V>(path, V::MAX_SUBPATH > 0)?;
    // SAFETY: as in `take`.
    let b = unsafe { Box::from_raw(p.as_ptr()) };
    let any: Box<dyn Any> = b;
    any.downcast::<V>().ok().map(|b| *b)
}

/// Removes the value at `path`. No host read. A decoded value is buried until the flush, not
/// freed (invariant 2).
pub(crate) fn delete<V: Value>(path: &Path) {
    bury(delete_impl::<V>(path, false));
}

/// [`delete`] for a map entry. If `V` owns storage and the entry was not read in this call,
/// the flush also removes its flag key, since the record may be mid-clear (D9): 1 more remove.
pub(crate) fn delete_entry<V: Value>(path: &Path) {
    bury(delete_impl::<V>(path, V::MAX_SUBPATH > 0));
}

fn bury(p: Option<NonNull<dyn Erased>>) {
    if let Some(p) = p {
        with(|c| c.graveyard.push(p));
    }
}

/// Marks the slot deleted and unlinks its value, which the caller takes or buries. A value that
/// owns storage is logged as gone (audit A2).
fn delete_impl<V: Value>(path: &Path, flaggable: bool) -> Option<NonNull<dyn Erased>> {
    let mut sp = with(|c| {
        if V::MAX_SUBPATH > 0 {
            c.record_gone(path.as_bytes());
        }
        c.entry_slot(path.as_bytes(), flaggable)
    });
    // SAFETY: invariants 1 and 2.
    let s = unsafe { sp.as_mut() };
    if s.flaggable && s.orig == Orig::Unknown {
        s.blind_delete = true;
    }
    s.mode = Mode::Deleted;
    s.clearing = Clearing::Cleared;
    s.verbatim = false;
    s.value.take()
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
        for (key, sp) in keys {
            if Some(&key) == root_key.as_ref() {
                continue;
            }
            // SAFETY: flush runs after the method body; invariant 1.
            let s = unsafe { sp.as_ref() };
            // A flag key sorts right after its record key and before every child of the
            // record, so emitting it here, after any op on the record key, keeps the flush in
            // key order (`flush_ops` emits the record key first).
            for op in s.flush_ops(&key) {
                match op {
                    FlushOp::Write(k, v) => c.host.write(&k, &v),
                    FlushOp::Remove(k) => c.host.remove(&k),
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
            !unsafe { s.as_ref() }.flush_ops(k).is_empty()
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
