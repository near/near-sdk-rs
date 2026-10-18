//! Reference model of storage v2, the oracle for the `verify` harnesses.
//!
//! Written from the spec (docs/storage-v2-spec.md), not from the implementation, in three
//! independent layers:
//!
//! 1. [`spec_key`]: the key grammar (root byte, LEB128 field tags, kind bytes,
//!    `h16(borsh k) ++ borsh k` entries, little-endian `u32` slots), without `key::Path`.
//! 2. [`MState`]: the abstract contract state (a counter, a key-vector map, a lookup map, a map
//!    of records owning a map, a vector) and [`MState::concretize`], the bytes the spec says
//!    that state occupies in storage.
//! 3. [`CacheModel`]: the spec's per-call cache rule ("each key is read at most once per call,
//!    written once at the end in key order with the root last; a value is rewritten only if its
//!    encoding changed"), which turns each operation's abstract accesses into host calls.
//!
//! Every operation's semantics emits the accesses the spec's cost table names for it. Two
//! derivations of the post-call storage must then agree: replaying the predicted host writes
//! and removes over the old storage, and `concretize` of the new abstract state. That
//! self-check makes the model fail on its own omissions before it is compared with the crate.
//!
//! The one rule taken from the prototype rather than the spec: "encoding changed" is decided
//! against the old bytes only when this call read them (a blind `insert` always writes). The
//! existing test `rewriting_the_same_value_is_elided_only_when_the_old_bytes_are_known` states it.

use crate::state::testing::OpKind;
use bolero::generator::TypeGenerator;
use std::collections::BTreeMap;
use std::vec::Vec;

// ---------------------------------------------------------------------------------------
// 1. Key grammar
// ---------------------------------------------------------------------------------------

pub mod spec_key {
    use near_digest::Digest;
    use std::vec::Vec;

    pub const ROOT: u8 = 0xF0;
    pub const ENTRIES: u8 = 0x00;
    pub const INDEX: u8 = 0x01;
    pub const SLOTS: u8 = 0x02;
    pub const HEADER: u8 = 0x03;
    pub const HASH_LEN: usize = crate::state::key::ENTRY_HASH_LEN;

    /// One path segment, as the spec's segment table lists them.
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub enum Seg {
        Field(u32),
        Kind(u8),
        /// A map entry: `borsh(k)`; the hash is derived when encoding.
        Entry(Vec<u8>),
        Slot(u32),
    }

    pub fn leb128(mut v: u64, out: &mut Vec<u8>) {
        loop {
            let b = (v & 0x7f) as u8;
            v >>= 7;
            if v == 0 {
                out.push(b);
                return;
            }
            out.push(b | 0x80);
        }
    }

    pub fn h(enc: &[u8]) -> Vec<u8> {
        near_digest::sha2::Sha256::digest(enc)[..HASH_LEN].to_vec()
    }

    pub fn encode(segs: &[Seg]) -> Vec<u8> {
        let mut out = vec![ROOT];
        for s in segs {
            match s {
                Seg::Field(t) => leb128(*t as u64, &mut out),
                Seg::Kind(k) => out.push(*k),
                Seg::Entry(enc) => {
                    out.extend_from_slice(&h(enc));
                    out.extend_from_slice(enc);
                }
                Seg::Slot(i) => out.extend_from_slice(&i.to_le_bytes()),
            }
        }
        out
    }

    /// The four tables of an iterable map rooted at `base`, keyed by a `u8`.
    #[derive(Clone, Debug)]
    pub struct MapKeys(pub Vec<Seg>);
    impl MapKeys {
        fn with(&self, extra: &[Seg]) -> Vec<u8> {
            let mut s = self.0.clone();
            s.extend_from_slice(extra);
            encode(&s)
        }
        pub fn entry(&self, k: u8) -> Vec<u8> {
            self.with(&[Seg::Kind(ENTRIES), Seg::Entry(vec![k])])
        }
        pub fn entry_segs(&self, k: u8) -> Vec<Seg> {
            let mut s = self.0.clone();
            s.extend_from_slice(&[Seg::Kind(ENTRIES), Seg::Entry(vec![k])]);
            s
        }
        pub fn index(&self, k: u8) -> Vec<u8> {
            self.with(&[Seg::Kind(INDEX), Seg::Entry(vec![k])])
        }
        pub fn slot(&self, i: u32) -> Vec<u8> {
            self.with(&[Seg::Kind(SLOTS), Seg::Slot(i)])
        }
        /// `Vector` keeps its slots under the entries kind.
        pub fn vslot(&self, i: u32) -> Vec<u8> {
            self.with(&[Seg::Kind(ENTRIES), Seg::Slot(i)])
        }
        pub fn header(&self) -> Vec<u8> {
            self.with(&[Seg::Kind(HEADER)])
        }
    }
}

use spec_key::{MapKeys, Seg};

pub const TAG_COUNTER: u32 = 1;
pub const TAG_FLAT: u32 = 2;
pub const TAG_LOOK: u32 = 3;
pub const TAG_RECS: u32 = 4;
pub const TAG_VEC: u32 = 5;
pub const REC_N: u32 = 1;
pub const REC_ITEMS: u32 = 2;
pub const REC2_NOTE: u32 = 3;
pub const REC2_NOTE_DEFAULT: u32 = 7;
pub const CLEARING_FLAG: [u8; 2] = [0, 0];

pub fn root_key() -> Vec<u8> {
    spec_key::encode(&[])
}
pub fn flat() -> MapKeys {
    MapKeys(vec![Seg::Field(TAG_FLAT)])
}
pub fn look() -> MapKeys {
    MapKeys(vec![Seg::Field(TAG_LOOK)])
}
pub fn recs() -> MapKeys {
    MapKeys(vec![Seg::Field(TAG_RECS)])
}
pub fn items(k: u8) -> MapKeys {
    let mut s = recs().entry_segs(k);
    s.push(Seg::Field(REC_ITEMS));
    MapKeys(s)
}
pub fn vect() -> MapKeys {
    MapKeys(vec![Seg::Field(TAG_VEC)])
}

// ---------------------------------------------------------------------------------------
// Value encodings (spec: leaves are Borsh, records are tag/len/bytes in ascending tag order)
// ---------------------------------------------------------------------------------------

pub fn enc_bytes(v: &[u8]) -> Vec<u8> {
    let mut out = (v.len() as u32).to_le_bytes().to_vec();
    out.extend_from_slice(v);
    out
}
pub fn enc_u32(v: u32) -> Vec<u8> {
    v.to_le_bytes().to_vec()
}
pub fn enc_record(fields: &BTreeMap<u32, Vec<u8>>, clearing: bool) -> Vec<u8> {
    let mut out = if clearing { CLEARING_FLAG.to_vec() } else { Vec::new() };
    for (t, b) in fields {
        spec_key::leb128(*t as u64, &mut out);
        spec_key::leb128(b.len() as u64, &mut out);
        out.extend_from_slice(b);
    }
    out
}

// ---------------------------------------------------------------------------------------
// 2. Abstract state
// ---------------------------------------------------------------------------------------

/// A `Rec`: its inline fields by tag (known or not to the code that reads it), the map it owns
/// in key-vector order, and the D9 clearing flag.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RecM {
    pub fields: BTreeMap<u32, Vec<u8>>,
    pub items: Vec<(u8, Vec<u8>)>,
    pub clearing: bool,
}
impl RecM {
    pub fn new(n: u64) -> Self {
        let mut fields = BTreeMap::new();
        fields.insert(REC_N, n.to_le_bytes().to_vec());
        RecM { fields, items: Vec::new(), clearing: false }
    }
    pub fn n(&self) -> u64 {
        self.fields.get(&REC_N).map_or(0, |b| u64::from_le_bytes(b[..].try_into().unwrap()))
    }
    pub fn set_n(&mut self, n: u64) {
        self.fields.insert(REC_N, n.to_le_bytes().to_vec());
    }
    pub fn note(&self) -> u32 {
        self.fields
            .get(&REC2_NOTE)
            .map_or(REC2_NOTE_DEFAULT, |b| u32::from_le_bytes(b[..].try_into().unwrap()))
    }
    pub fn bytes(&self) -> Vec<u8> {
        enc_record(&self.fields, self.clearing)
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MState {
    pub root_written: bool,
    pub counter: u64,
    /// `Map<u8, Vec<u8>>` in key-vector order (insertion order until a remove moves the last
    /// key into the hole).
    pub flat: Vec<(u8, Vec<u8>)>,
    pub look: BTreeMap<u8, Vec<u8>>,
    pub recs: Vec<(u8, RecM)>,
    pub vec: Vec<Vec<u8>>,
}

fn pos<V>(v: &[(u8, V)], k: u8) -> Option<usize> {
    v.iter().position(|(x, _)| *x == k)
}

fn concretize_map(out: &mut BTreeMap<Vec<u8>, Vec<u8>>, mk: &MapKeys, v: &[(u8, Vec<u8>)]) {
    for (i, (k, val)) in v.iter().enumerate() {
        out.insert(mk.entry(*k), enc_bytes(val));
        out.insert(mk.index(*k), enc_u32(i as u32));
        out.insert(mk.slot(i as u32), vec![*k]);
    }
    if !v.is_empty() {
        out.insert(mk.header(), enc_u32(v.len() as u32));
    }
}

impl MState {
    pub fn root_bytes(&self) -> Vec<u8> {
        let mut f = BTreeMap::new();
        f.insert(TAG_COUNTER, self.counter.to_le_bytes().to_vec());
        enc_record(&f, false)
    }

    /// The storage this state occupies, per the spec's key grammar and value encodings.
    pub fn concretize(&self) -> BTreeMap<Vec<u8>, Vec<u8>> {
        let mut out = BTreeMap::new();
        if self.root_written {
            out.insert(root_key(), self.root_bytes());
        }
        concretize_map(&mut out, &flat(), &self.flat);
        for (k, v) in &self.look {
            out.insert(look().entry(*k), enc_bytes(v));
        }
        let r = recs();
        for (i, (k, rec)) in self.recs.iter().enumerate() {
            out.insert(r.entry(*k), rec.bytes());
            out.insert(r.index(*k), enc_u32(i as u32));
            out.insert(r.slot(i as u32), vec![*k]);
            concretize_map(&mut out, &items(*k), &rec.items);
        }
        if !self.recs.is_empty() {
            out.insert(r.header(), enc_u32(self.recs.len() as u32));
        }
        let vk = vect();
        for (i, v) in self.vec.iter().enumerate() {
            out.insert(vk.vslot(i as u32), enc_bytes(v));
        }
        if !self.vec.is_empty() {
            out.insert(vk.header(), enc_u32(self.vec.len() as u32));
        }
        out
    }
}

// ---------------------------------------------------------------------------------------
// 3. The per-call cache, at the level of the spec's rule
// ---------------------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Eq)]
enum Orig {
    Unknown,
    Absent,
    PresentUnread,
    Present(Vec<u8>),
}

#[derive(Clone, Debug)]
struct MSlot {
    orig: Orig,
    /// `None`: not changed in this call. `Some(None)`: removed. `Some(Some(b))`: will hold `b`.
    pending: Option<Option<Vec<u8>>>,
}

/// Host calls the model predicts for one call.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Predicted {
    /// Reads and `storage_has_key`s, as `(kind, key, value_len)`; compared as a multiset.
    pub lookups: Vec<(OpKind, Vec<u8>, usize)>,
    /// Writes and removes in flush order, as `(kind, key, value_len, replaced_len)`.
    pub mutations: Vec<(OpKind, Vec<u8>, usize, Option<usize>)>,
}

pub struct CacheModel {
    /// The host's storage (committed state). Changes only at a flush.
    pub host: BTreeMap<Vec<u8>, Vec<u8>>,
    slots: BTreeMap<Vec<u8>, MSlot>,
    pub predicted: Predicted,
}

impl CacheModel {
    pub fn new(host: BTreeMap<Vec<u8>, Vec<u8>>) -> Self {
        CacheModel { host, slots: BTreeMap::new(), predicted: Predicted::default() }
    }

    fn slot(&mut self, key: &[u8]) -> &mut MSlot {
        self.slots.entry(key.to_vec()).or_insert(MSlot { orig: Orig::Unknown, pending: None })
    }

    /// The current bytes at `key`; a host read at most once per call, never after this call
    /// changed the key.
    pub fn read(&mut self, key: &[u8]) -> Option<Vec<u8>> {
        let host = self.host.get(key).cloned();
        let s = self.slots.entry(key.to_vec()).or_insert(MSlot { orig: Orig::Unknown, pending: None });
        if let Some(p) = &s.pending {
            return p.clone();
        }
        match &s.orig {
            Orig::Present(b) => Some(b.clone()),
            Orig::Absent => None,
            Orig::Unknown | Orig::PresentUnread => {
                self.predicted.lookups.push((
                    OpKind::Read,
                    key.to_vec(),
                    host.as_ref().map_or(0, Vec::len),
                ));
                s.orig = match &host {
                    Some(b) => Orig::Present(b.clone()),
                    None => Orig::Absent,
                };
                host
            }
        }
    }

    /// Whether a value exists at `key`: from the cache, else one `storage_has_key`.
    pub fn exists(&mut self, key: &[u8]) -> bool {
        let present = self.host.contains_key(key);
        let s = self.slots.entry(key.to_vec()).or_insert(MSlot { orig: Orig::Unknown, pending: None });
        if let Some(p) = &s.pending {
            return p.is_some();
        }
        match &s.orig {
            Orig::Present(_) | Orig::PresentUnread => true,
            Orig::Absent => false,
            Orig::Unknown => {
                self.predicted.lookups.push((OpKind::HasKey, key.to_vec(), 0));
                s.orig = if present { Orig::PresentUnread } else { Orig::Absent };
                present
            }
        }
    }

    pub fn write(&mut self, key: &[u8], bytes: Vec<u8>) {
        self.slot(key).pending = Some(Some(bytes));
    }

    pub fn delete(&mut self, key: &[u8]) {
        self.slot(key).pending = Some(None);
    }

    fn host_write(&mut self, key: &[u8], b: Vec<u8>) {
        let old = self.host.insert(key.to_vec(), b.clone());
        self.predicted.mutations.push((OpKind::Write, key.to_vec(), b.len(), old.map(|o| o.len())));
    }

    /// Every changed key once, in key order, then the root last (written iff its bytes differ
    /// from the bytes this call read for it); then the cache is empty.
    pub fn flush(&mut self, root: Option<Vec<u8>>) {
        let rk = root_key();
        let slots = core::mem::take(&mut self.slots);
        for (key, s) in &slots {
            if *key == rk {
                continue;
            }
            match &s.pending {
                None => {}
                Some(Some(b)) => {
                    if s.orig != Orig::Present(b.clone()) {
                        self.host_write(key, b.clone());
                    }
                }
                Some(None) => {
                    if s.orig != Orig::Absent {
                        let old = self.host.remove(key);
                        self.predicted.mutations.push((
                            OpKind::Remove,
                            key.clone(),
                            old.as_ref().map_or(0, Vec::len),
                            old.map(|o| o.len()),
                        ));
                    }
                }
            }
        }
        if let Some(b) = root {
            let orig = slots.get(&rk).map(|s| s.orig.clone()).unwrap_or(Orig::Unknown);
            if orig != Orig::Present(b.clone()) {
                self.host_write(&rk, b);
            }
        }
    }

    /// Drops the cache without flushing (an aborted call).
    pub fn abort(&mut self) {
        self.slots.clear();
    }

    pub fn take_predicted(&mut self) -> Predicted {
        core::mem::take(&mut self.predicted)
    }
}

// ---------------------------------------------------------------------------------------
// Operations
// ---------------------------------------------------------------------------------------

/// A key in a small domain, so that operations hit each other.
#[derive(Clone, Copy, Debug, PartialEq, Eq, TypeGenerator)]
pub struct K(#[generator(0..5)] pub u8);

/// A value: `len` copies of `fill`, from a small domain so that equal rewrites happen.
#[derive(Clone, Copy, Debug, PartialEq, Eq, TypeGenerator)]
pub struct Val {
    #[generator(0..6)]
    pub len: u8,
    #[generator(0..3)]
    pub fill: u8,
}
impl Val {
    pub fn bytes(&self) -> Vec<u8> {
        vec![self.fill; self.len as usize]
    }
}

/// One step of a generated contract run. `EndCall` ends the call (flush, then a new call).
#[derive(Clone, Debug, PartialEq, Eq, TypeGenerator)]
pub enum Op {
    EndCall,
    /// `state_write()` by hand in the middle of a method (no references held), then the
    /// method continues.
    MidFlush,
    /// A view call of its own: reads `flat[k]`, `look[k]`, `recs[k].n`, `recs.at(k).items.len()`.
    View(K),
    Counter(#[generator(0..3)] u8),
    FlatInsert(K, Val),
    FlatGet(K),
    FlatContains(K),
    FlatGetMut(K, Option<u8>),
    FlatEntry(K, Val),
    FlatRemove(K),
    FlatDiscard(K),
    FlatPage(Option<K>, #[generator(0..4)] u8),
    FlatKeysPage(Option<K>, #[generator(0..4)] u8),
    FlatIter(#[generator(0..4)] u8, #[generator(0..4)] u8),
    FlatLen,
    LookInsert(K, Val),
    LookGet(K),
    LookContains(K),
    LookGetMut(K, Option<u8>),
    LookEntry(K, Val),
    LookRemove(K),
    LookDiscard(K),
    RecInsertNew(K, #[generator(0..3)] u8),
    RecEntryDefault(K),
    RecGetMut(K, #[generator(0..3)] u8),
    RecGet(K),
    RecContains(K),
    RecAtMutInsert(K, K, Val),
    RecAtMutRemove(K, K),
    RecAtLen(K),
    RecRemove(K),
    RecRemoveSome(K, #[generator(0..4)] u8),
    RecPage(Option<K>, #[generator(0..4)] u8),
    /// In a call of its own: the record map read as `Map<u8, RecV2>` (tag 1 removed, tag 3
    /// added with `default = 7`), `get_mut(k).note += x`.
    RecRetyped(K, #[generator(0..3)] u8),
    VecPush(Val),
    VecPop,
    VecSwapRemove(#[generator(0..5)] u8),
    VecGet(#[generator(0..5)] u8),
    VecGetMut(#[generator(0..5)] u8, Option<u8>),
    VecLen,
}

/// What an operation returned, in a form both sides can produce.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Ret {
    Unit,
    Bool(bool),
    U32(u32),
    U64(u64),
    Bytes(Option<Vec<u8>>),
    OptU64(Option<u64>),
    /// `Ok((items, next))` or `Err(CursorGone)`.
    Page(Result<(Vec<(u8, Vec<u8>)>, Option<u8>), ()>),
    KeyPage(Result<(Vec<u8>, Option<u8>), ()>),
    RecPage(Result<(Vec<(u8, u64)>, Option<u8>), ()>),
    Items(Vec<(u8, Vec<u8>)>),
    View(Option<Vec<u8>>, Option<Vec<u8>>, Option<u64>, u32),
}

// ---------------------------------------------------------------------------------------
// Semantics
// ---------------------------------------------------------------------------------------

/// Expected-value check inside the model: the cache's view of a key must match the abstract
/// state's. Failing here is a model bug, reported as such.
fn agree(what: &str, cache: Option<Vec<u8>>, abs: Option<Vec<u8>>) {
    assert_eq!(cache, abs, "MODEL BUG: cache view and abstract state disagree on {what}");
}

/// Map key-vector operations shared by `flat`, `recs` and every `items` map.
struct KV<'a, V> {
    mk: MapKeys,
    v: &'a mut Vec<(u8, V)>,
}

impl<V: Clone> KV<'_, V> {
    fn len(&self, c: &mut CacheModel) -> u32 {
        let h = c.read(&self.mk.header());
        let n = self.v.len() as u32;
        agree("map header", h, if n == 0 { None } else { Some(enc_u32(n)) });
        n
    }
    fn set_len(&self, c: &mut CacheModel, n: u32) {
        if n == 0 { c.delete(&self.mk.header()) } else { c.write(&self.mk.header(), enc_u32(n)) }
    }
    fn index(&self, c: &mut CacheModel, k: u8) -> Option<u32> {
        let r = c.read(&self.mk.index(k));
        let p = pos(self.v, k).map(|i| i as u32);
        agree("map index", r, p.map(enc_u32));
        p
    }
    fn slot_key(&self, c: &mut CacheModel, i: u32) -> u8 {
        let r = c.read(&self.mk.slot(i));
        let k = self.v[i as usize].0;
        agree("key slot", r, Some(vec![k]));
        k
    }
    /// Appends `k` (its value is written by the caller): header read, index, slot, header.
    fn push_key(&mut self, c: &mut CacheModel, k: u8, v: V) {
        let n = self.len(c);
        c.write(&self.mk.index(k), enc_u32(n));
        c.write(&self.mk.slot(n), vec![k]);
        self.set_len(c, n + 1);
        self.v.push((k, v));
    }
    /// The swap-remove tail of `Map::remove` after the entry is gone; the caller has checked
    /// that `k` is present. Index read, header read, moved key, header.
    fn unlink(&mut self, c: &mut CacheModel, k: u8) {
        let i = self.index(c, k).expect("present");
        let n = self.len(c);
        c.delete(&self.mk.entry(k));
        c.delete(&self.mk.index(k));
        let last = n - 1;
        if i != last {
            let moved = self.slot_key(c, last);
            c.write(&self.mk.slot(i), vec![moved]);
            c.write(&self.mk.index(moved), enc_u32(i));
        }
        c.delete(&self.mk.slot(last));
        self.set_len(c, last);
        self.v.swap_remove(i as usize);
    }
    /// `clear()`: header read, then per entry from the end a slot read and three removes.
    fn clear(&mut self, c: &mut CacheModel) {
        let n = self.len(c);
        for i in (0..n).rev() {
            let k = self.slot_key(c, i);
            c.delete(&self.mk.entry(k));
            c.delete(&self.mk.index(k));
            c.delete(&self.mk.slot(i));
        }
        self.set_len(c, 0);
        self.v.clear();
    }
    /// Bounded clear from the end (D9). Returns whether the map is now empty. When the budget
    /// runs out with entries left, the next slot has already been read.
    fn clear_budgeted(&mut self, c: &mut CacheModel, budget: &mut u32) -> bool {
        loop {
            let n = self.len(c);
            if n == 0 {
                return true;
            }
            let k = self.slot_key(c, n - 1);
            if *budget == 0 {
                return false;
            }
            *budget -= 1;
            c.delete(&self.mk.entry(k));
            c.delete(&self.mk.index(k));
            c.delete(&self.mk.slot(n - 1));
            self.set_len(c, n - 1);
            self.v.pop();
        }
    }
    fn page_bounds(&self, c: &mut CacheModel, after: Option<u8>, limit: u8) -> Result<(u32, u32, u32), ()> {
        let start = match after {
            None => 0,
            Some(k) => self.index(c, k).ok_or(())? + 1,
        };
        let n = self.len(c);
        let end = start.saturating_add(limit as u32).min(n);
        Ok((start, end, n))
    }
}

/// Leaf-valued map operations (`flat` and the `items` maps).
impl KV<'_, Vec<u8>> {
    fn get(&self, c: &mut CacheModel, k: u8) -> Option<Vec<u8>> {
        let r = c.read(&self.mk.entry(k));
        let v = pos(self.v, k).map(|i| self.v[i].1.clone());
        agree("map entry", r, v.as_deref().map(enc_bytes));
        v
    }
    fn insert(&mut self, c: &mut CacheModel, k: u8, val: Vec<u8>) -> bool {
        let new = self.index(c, k).is_none();
        c.write(&self.mk.entry(k), enc_bytes(&val));
        if new {
            self.push_key(c, k, val);
        } else {
            let i = pos(self.v, k).unwrap();
            self.v[i].1 = val;
        }
        new
    }
    fn remove(&mut self, c: &mut CacheModel, k: u8, read_value: bool) -> Option<Vec<u8>> {
        if read_value {
            self.get(c, k)?;
        } else {
            self.index(c, k)?;
        }
        let v = self.v[pos(self.v, k).unwrap()].1.clone();
        self.unlink(c, k);
        Some(v)
    }
    fn page(&self, c: &mut CacheModel, after: Option<u8>, limit: u8, keys_only: bool) -> Result<(Vec<(u8, Vec<u8>)>, Option<u8>), ()> {
        let (start, end, n) = self.page_bounds(c, after, limit)?;
        let mut items = Vec::new();
        for i in start..end {
            let k = self.slot_key(c, i);
            let v = if keys_only { Vec::new() } else { self.get(c, k).expect("indexed") };
            items.push((k, v));
        }
        let next = if end < n { items.last().map(|(k, _)| *k) } else { None };
        Ok((items, next))
    }
}

/// The model of one contract over the fixture schema.
pub struct Model {
    pub committed: MState,
    pub cur: MState,
    pub cache: CacheModel,
}

impl Default for Model {
    fn default() -> Self {
        Model { committed: MState::default(), cur: MState::default(), cache: CacheModel::new(BTreeMap::new()) }
    }
}

impl Model {
    /// Starts a call: the root is read and decoded.
    pub fn begin_call(&mut self) {
        let r = self.cache.read(&root_key());
        agree("root", r, self.committed.root_written.then(|| self.committed.root_bytes()));
    }

    /// Ends a mutating call: flush (root last), check the model against itself, commit.
    pub fn end_call(&mut self) -> Predicted {
        self.flush_only();
        self.cache.take_predicted()
    }

    fn flush_only(&mut self) {
        self.cur.root_written = true;
        self.cache.flush(Some(self.cur.root_bytes()));
        assert_eq!(
            self.cache.host,
            self.cur.concretize(),
            "MODEL BUG: replaying the predicted host calls and concretizing the abstract state disagree"
        );
        self.committed = self.cur.clone();
    }

    /// `state_write()` by hand mid-method: flush, keep going in the same receipt.
    pub fn mid_flush(&mut self) {
        self.flush_only();
    }

    /// Ends a view call: nothing may be written.
    pub fn end_view(&mut self) -> Predicted {
        self.cache.abort();
        self.cache.take_predicted()
    }

    #[allow(dead_code)] // used by the failure-atomicity harness
    /// Aborts the call (a panic): the cache is dropped, storage is untouched.
    pub fn abort_call(&mut self) -> Predicted {
        self.cache.abort();
        self.cur = self.committed.clone();
        self.cache.take_predicted()
    }

    fn rec_pos(&self, k: u8) -> Option<usize> {
        pos(&self.cur.recs, k)
    }

    /// Whether the implementation is expected to panic on `op` (the harness then skips it;
    /// dedicated harnesses check the panics).
    pub fn would_panic(&self, op: &Op) -> bool {
        let clearing = |k: u8| self.rec_pos(k).is_some_and(|i| self.cur.recs[i].1.clearing);
        match *op {
            Op::RecInsertNew(K(k), _) => self.rec_pos(k).is_some(),
            Op::RecEntryDefault(K(k)) | Op::RecGetMut(K(k), _) | Op::RecRetyped(K(k), _) => clearing(k),
            Op::RecAtMutInsert(K(k), ..) | Op::RecAtMutRemove(K(k), _) => clearing(k),
            Op::VecSwapRemove(i) => i as usize >= self.cur.vec.len(),
            _ => false,
        }
    }

    fn rec_get(&mut self, k: u8) -> Option<usize> {
        let r = self.cache.read(&recs().entry(k));
        let i = self.rec_pos(k);
        agree("record", r, i.map(|i| self.cur.recs[i].1.bytes()));
        i
    }

    /// Applies `op` inside the current call. Call-boundary ops are handled by the driver.
    pub fn apply(&mut self, op: &Op) -> Ret {
        let c = &mut self.cache;
        match *op {
            Op::EndCall | Op::MidFlush | Op::View(_) | Op::RecRetyped(..) => {
                unreachable!("driver handles call boundaries")
            }
            Op::Counter(x) => {
                self.cur.counter = self.cur.counter.wrapping_add(x as u64);
                Ret::Unit
            }
            Op::FlatInsert(K(k), v) => Ret::Bool(KV { mk: flat(), v: &mut self.cur.flat }.insert(c, k, v.bytes())),
            Op::FlatGet(K(k)) => Ret::Bytes(KV { mk: flat(), v: &mut self.cur.flat }.get(c, k)),
            Op::FlatContains(K(k)) => {
                let e = c.exists(&flat().entry(k));
                assert_eq!(e, pos(&self.cur.flat, k).is_some(), "MODEL BUG: contains");
                Ret::Bool(e)
            }
            Op::FlatGetMut(K(k), x) => {
                let m = KV { mk: flat(), v: &mut self.cur.flat };
                match m.get(c, k) {
                    None => Ret::Bytes(None),
                    Some(mut v) => {
                        if let Some(b) = x {
                            v.push(b);
                        }
                        c.write(&flat().entry(k), enc_bytes(&v));
                        let i = pos(&self.cur.flat, k).unwrap();
                        self.cur.flat[i].1 = v.clone();
                        Ret::Bytes(Some(v))
                    }
                }
            }
            Op::FlatEntry(K(k), v) => {
                let mut m = KV { mk: flat(), v: &mut self.cur.flat };
                match m.get(c, k) {
                    Some(old) => Ret::Bytes(Some(old)),
                    None => {
                        c.write(&flat().entry(k), enc_bytes(&v.bytes()));
                        m.push_key(c, k, v.bytes());
                        Ret::Bytes(Some(v.bytes()))
                    }
                }
            }
            Op::FlatRemove(K(k)) => Ret::Bytes(KV { mk: flat(), v: &mut self.cur.flat }.remove(c, k, true)),
            Op::FlatDiscard(K(k)) => {
                Ret::Bool(KV { mk: flat(), v: &mut self.cur.flat }.remove(c, k, false).is_some())
            }
            Op::FlatPage(after, limit) => {
                Ret::Page(KV { mk: flat(), v: &mut self.cur.flat }.page(c, after.map(|k| k.0), limit, false))
            }
            Op::FlatKeysPage(after, limit) => Ret::KeyPage(
                KV { mk: flat(), v: &mut self.cur.flat }
                    .page(c, after.map(|k| k.0), limit, true)
                    .map(|(items, next)| (items.into_iter().map(|(k, _)| k).collect(), next)),
            ),
            Op::FlatIter(skip, take) => {
                let m = KV { mk: flat(), v: &mut self.cur.flat };
                let n = m.len(c);
                let mut out = Vec::new();
                for i in (skip as u32)..n.min(skip as u32 + take as u32) {
                    let k = m.slot_key(c, i);
                    out.push((k, m.get(c, k).expect("indexed")));
                }
                Ret::Items(out)
            }
            Op::FlatLen => Ret::U32(KV { mk: flat(), v: &mut self.cur.flat }.len(c)),
            Op::LookInsert(K(k), v) => {
                c.write(&look().entry(k), enc_bytes(&v.bytes()));
                self.cur.look.insert(k, v.bytes());
                Ret::Unit
            }
            Op::LookGet(K(k)) => {
                let r = c.read(&look().entry(k));
                let v = self.cur.look.get(&k).cloned();
                agree("lookup entry", r, v.as_deref().map(enc_bytes));
                Ret::Bytes(v)
            }
            Op::LookContains(K(k)) => {
                let e = c.exists(&look().entry(k));
                assert_eq!(e, self.cur.look.contains_key(&k), "MODEL BUG: lookup contains");
                Ret::Bool(e)
            }
            Op::LookGetMut(K(k), x) => {
                let r = c.read(&look().entry(k));
                agree("lookup entry", r, self.cur.look.get(&k).map(|v| enc_bytes(v)));
                match self.cur.look.get_mut(&k) {
                    None => Ret::Bytes(None),
                    Some(v) => {
                        if let Some(b) = x {
                            v.push(b);
                        }
                        c.write(&look().entry(k), enc_bytes(v));
                        Ret::Bytes(Some(v.clone()))
                    }
                }
            }
            Op::LookEntry(K(k), v) => {
                let r = c.read(&look().entry(k));
                agree("lookup entry", r, self.cur.look.get(&k).map(|v| enc_bytes(v)));
                match self.cur.look.get(&k) {
                    Some(old) => Ret::Bytes(Some(old.clone())),
                    None => {
                        c.write(&look().entry(k), enc_bytes(&v.bytes()));
                        self.cur.look.insert(k, v.bytes());
                        Ret::Bytes(Some(v.bytes()))
                    }
                }
            }
            Op::LookRemove(K(k)) => {
                let r = c.read(&look().entry(k));
                agree("lookup entry", r, self.cur.look.get(&k).map(|v| enc_bytes(v)));
                let old = self.cur.look.remove(&k);
                if old.is_some() {
                    c.delete(&look().entry(k));
                }
                Ret::Bytes(old)
            }
            Op::LookDiscard(K(k)) => {
                c.delete(&look().entry(k));
                self.cur.look.remove(&k);
                Ret::Unit
            }
            Op::RecInsertNew(K(k), n) => {
                let mut m = KV { mk: recs(), v: &mut self.cur.recs };
                assert!(m.index(c, k).is_none(), "skipped by would_panic");
                let rec = RecM::new(n as u64);
                c.write(&recs().entry(k), rec.bytes());
                m.push_key(c, k, rec);
                Ret::Unit
            }
            Op::RecEntryDefault(K(k)) => match self.rec_get(k) {
                Some(i) => Ret::U64(self.cur.recs[i].1.n()),
                None => {
                    let rec = RecM::new(0);
                    let c = &mut self.cache;
                    c.write(&recs().entry(k), rec.bytes());
                    KV { mk: recs(), v: &mut self.cur.recs }.push_key(c, k, rec);
                    Ret::U64(0)
                }
            },
            Op::RecGetMut(K(k), x) => match self.rec_get(k) {
                None => Ret::OptU64(None),
                Some(i) => {
                    let r = &mut self.cur.recs[i].1;
                    let n = r.n().wrapping_add(x as u64);
                    r.set_n(n);
                    let b = r.bytes();
                    self.cache.write(&recs().entry(k), b);
                    Ret::OptU64(Some(n))
                }
            },
            Op::RecGet(K(k)) => {
                let i = self.rec_get(k);
                Ret::OptU64(i.map(|i| self.cur.recs[i].1.n()))
            }
            Op::RecContains(K(k)) => {
                let e = c.exists(&recs().entry(k));
                assert_eq!(e, self.rec_pos(k).is_some(), "MODEL BUG: record contains");
                Ret::Bool(e)
            }
            Op::RecAtMutInsert(K(k), K(k2), v) => {
                // `at_mut`: 1 read of the record bytes (no decode), `None` if absent.
                let Some(i) = self.rec_get(k) else { return Ret::OptU64(None) };
                let c = &mut self.cache;
                let new = KV { mk: items(k), v: &mut self.cur.recs[i].1.items }.insert(c, k2, v.bytes());
                Ret::Bool(new)
            }
            Op::RecAtMutRemove(K(k), K(k2)) => {
                let Some(i) = self.rec_get(k) else { return Ret::OptU64(None) };
                let c = &mut self.cache;
                Ret::Bytes(KV { mk: items(k), v: &mut self.cur.recs[i].1.items }.remove(c, k2, true))
            }
            // `at(&k)` reads nothing; `.items.len()` reads the header (absent record: 0).
            Op::RecAtLen(K(k)) => Ret::U32(self.rec_items_len(k)),
            Op::RecRemove(K(k)) => {
                let Some(i) = self.rec_get(k) else { return Ret::OptU64(None) };
                let n = self.cur.recs[i].1.n();
                let c = &mut self.cache;
                KV { mk: items(k), v: &mut self.cur.recs[i].1.items }.clear(c);
                KV { mk: recs(), v: &mut self.cur.recs }.unlink(c, k);
                Ret::OptU64(Some(n))
            }
            Op::RecRemoveSome(K(k), n) => {
                let Some(i) = self.rec_get(k) else { return Ret::Bool(false) };
                let c = &mut self.cache;
                let mut budget = n as u32;
                let done = KV { mk: items(k), v: &mut self.cur.recs[i].1.items }.clear_budgeted(c, &mut budget);
                if done {
                    // `remove_inner(k, false)`: clear_children again (the header, already
                    // known), then unlink.
                    KV { mk: items(k), v: &mut self.cur.recs[i].1.items }.clear(c);
                    KV { mk: recs(), v: &mut self.cur.recs }.unlink(c, k);
                    Ret::Bool(false)
                } else {
                    let r = &mut self.cur.recs[i].1;
                    r.clearing = true;
                    let b = r.bytes();
                    c.write(&recs().entry(k), b);
                    Ret::Bool(true)
                }
            }
            Op::RecPage(after, limit) => {
                let m = KV { mk: recs(), v: &mut self.cur.recs };
                let r = m.page_bounds(c, after.map(|k| k.0), limit).map(|(start, end, n)| {
                    let mut out = Vec::new();
                    for i in start..end {
                        let k = m.slot_key(c, i);
                        let rb = c.read(&recs().entry(k));
                        let rec = &m.v[i as usize].1;
                        agree("record", rb, Some(rec.bytes()));
                        out.push((k, rec.n()));
                    }
                    let next = if end < n { out.last().map(|(k, _)| *k) } else { None };
                    (out, next)
                });
                Ret::RecPage(r)
            }
            Op::VecPush(v) => {
                let n = self.vec_len();
                let c = &mut self.cache;
                c.write(&vect().vslot(n), enc_bytes(&v.bytes()));
                c.write(&vect().header(), enc_u32(n + 1));
                self.cur.vec.push(v.bytes());
                Ret::Unit
            }
            Op::VecPop => {
                let n = self.vec_len();
                if n == 0 {
                    return Ret::Bytes(None);
                }
                let v = self.vec_get(n - 1).expect("dense");
                let c = &mut self.cache;
                c.delete(&vect().vslot(n - 1));
                if n == 1 { c.delete(&vect().header()) } else { c.write(&vect().header(), enc_u32(n - 1)) }
                self.cur.vec.pop();
                Ret::Bytes(Some(v))
            }
            Op::VecSwapRemove(i) => {
                let i = i as u32;
                let n = self.vec_len();
                assert!(i < n, "skipped by would_panic");
                let v = self.vec_get(i).expect("dense");
                self.cache.delete(&vect().vslot(i));
                let last = n - 1;
                if i != last {
                    let moved = self.vec_get(last).expect("dense");
                    self.cache.delete(&vect().vslot(last));
                    self.cache.write(&vect().vslot(i), enc_bytes(&moved));
                }
                let c = &mut self.cache;
                if last == 0 { c.delete(&vect().header()) } else { c.write(&vect().header(), enc_u32(last)) }
                self.cur.vec.swap_remove(i as usize);
                Ret::Bytes(Some(v))
            }
            Op::VecGet(i) => Ret::Bytes(self.vec_get(i as u32)),
            Op::VecGetMut(i, x) => match self.vec_get(i as u32) {
                None => Ret::Bytes(None),
                Some(mut v) => {
                    if let Some(b) = x {
                        v.push(b);
                    }
                    self.cache.write(&vect().vslot(i as u32), enc_bytes(&v));
                    self.cur.vec[i as usize] = v.clone();
                    Ret::Bytes(Some(v))
                }
            },
            Op::VecLen => Ret::U32(self.vec_len()),
        }
    }

    fn vec_len(&mut self) -> u32 {
        let h = self.cache.read(&vect().header());
        let n = self.cur.vec.len() as u32;
        agree("vector header", h, if n == 0 { None } else { Some(enc_u32(n)) });
        n
    }

    fn vec_get(&mut self, i: u32) -> Option<Vec<u8>> {
        let r = self.cache.read(&vect().vslot(i));
        let v = self.cur.vec.get(i as usize).cloned();
        agree("vector slot", r, v.as_deref().map(enc_bytes));
        v
    }

    /// A view call over the current committed state (own call: begin, reads, no flush).
    pub fn view(&mut self, k: u8) -> (Ret, Predicted) {
        self.begin_call();
        let a = KV { mk: flat(), v: &mut self.cur.flat }.get(&mut self.cache, k);
        let c = &mut self.cache;
        let r = c.read(&look().entry(k));
        let b = self.cur.look.get(&k).cloned();
        agree("lookup entry", r, b.as_deref().map(enc_bytes));
        let n = self.rec_get(k).map(|i| self.cur.recs[i].1.n());
        let len = self.rec_items_len(k);
        (Ret::View(a, b, n, len), self.end_view())
    }

    fn rec_items_len(&mut self, k: u8) -> u32 {
        let i = self.rec_pos(k);
        let mut empty = Vec::new();
        let v = match i {
            Some(i) => &mut self.cur.recs[i].1.items,
            None => &mut empty,
        };
        KV { mk: items(k), v }.len(&mut self.cache)
    }

    /// The re-typed call: `Map<u8, RecV2>::get_mut(k)`, `note += x`. Own call.
    pub fn retyped(&mut self, k: u8, x: u8) -> (Ret, Predicted) {
        self.begin_call();
        let ret = match self.rec_get(k) {
            None => Ret::OptU64(None),
            Some(i) => {
                let r = &mut self.cur.recs[i].1;
                let note = r.note().wrapping_add(x as u32);
                r.fields.insert(REC2_NOTE, note.to_le_bytes().to_vec());
                let b = r.bytes();
                self.cache.write(&recs().entry(k), b);
                Ret::OptU64(Some(note as u64))
            }
        };
        (ret, self.end_call())
    }
}
