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
//!
//! Since the merge with API rounds 2 to 4 (Oct 18) the model follows the D9 flag key, T4 and T1
//! as the spec states them: a record being cleared lives at `record_key ++ 00` with its record
//! key absent ([`RSlot`]), a miss on a record that owns storage consults the flag key (a read
//! for `get`/`entry`/`remove`/`remove_step`/pages, a `storage_has_key` for `contains_key` and
//! `at_mut`), `at_mut` checks existence and never reads, a `Map` header carries a removal
//! revision and survives emptying (it goes only with the record owning the map), and
//! `try_insert` is first-write-wins on every value type.
//!
//! Generation-stamped handles (API round 4) need no case here: every operation below takes a
//! fresh handle from the root (`root.recs.at_mut(&k)`), so none is ever stale. The escaped-handle
//! panic is checked by `verify::c5_remove_some` (the `Escaped` attempt).

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
/// The D9 flag segment: a record being cleared is stored at its key plus this byte.
pub const FLAG_SEGMENT: u8 = 0x00;

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
/// Where record `k` lives while `remove_step` is clearing it: its key plus `00`.
pub fn rec_flag(k: u8) -> Vec<u8> {
    flag_of(&recs().entry(k))
}
pub fn flag_of(record_key: &[u8]) -> Vec<u8> {
    let mut f = record_key.to_vec();
    f.push(FLAG_SEGMENT);
    f
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
/// A `Map` header (T4): the bare `u32` length while the revision is 0, `len ++ revision` (12
/// bytes) after the first removal.
pub fn enc_header(len: u32, rev: u64) -> Vec<u8> {
    let mut out = enc_u32(len);
    if rev != 0 {
        out.extend_from_slice(&rev.to_le_bytes());
    }
    out
}
/// Whether a `Map` header key exists: an empty map that never had a removal has none.
pub fn header_stored(len: u32, rev: u64) -> Option<Vec<u8>> {
    (len != 0 || rev != 0).then(|| enc_header(len, rev))
}
pub fn enc_record(fields: &BTreeMap<u32, Vec<u8>>) -> Vec<u8> {
    let mut out = Vec::new();
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
/// in key-vector order with that map's removal revision, and whether `remove_step` is clearing
/// it (D9: then its bytes live at the flag key and its record key is absent).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RecM {
    pub fields: BTreeMap<u32, Vec<u8>>,
    pub items: Vec<(u8, Vec<u8>)>,
    pub items_rev: u64,
    pub clearing: bool,
}
impl RecM {
    pub fn new(n: u64) -> Self {
        let mut fields = BTreeMap::new();
        fields.insert(REC_N, n.to_le_bytes().to_vec());
        RecM { fields, items: Vec::new(), items_rev: 0, clearing: false }
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
    /// The record's bytes, wherever they are stored (the flag carries no bytes of its own).
    pub fn bytes(&self) -> Vec<u8> {
        enc_record(&self.fields)
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MState {
    pub root_written: bool,
    pub counter: u64,
    /// `Map<u8, Vec<u8>>` in key-vector order (insertion order until a remove moves the last
    /// key into the hole).
    pub flat: Vec<(u8, Vec<u8>)>,
    /// `flat`'s header revision (T4): bumped by every removal, never reset.
    pub flat_rev: u64,
    pub look: BTreeMap<u8, Vec<u8>>,
    pub recs: Vec<(u8, RecM)>,
    pub recs_rev: u64,
    pub vec: Vec<Vec<u8>>,
}

fn pos<V>(v: &[(u8, V)], k: u8) -> Option<usize> {
    v.iter().position(|(x, _)| *x == k)
}

fn concretize_map(out: &mut BTreeMap<Vec<u8>, Vec<u8>>, mk: &MapKeys, v: &[(u8, Vec<u8>)], rev: u64) {
    for (i, (k, val)) in v.iter().enumerate() {
        out.insert(mk.entry(*k), enc_bytes(val));
        out.insert(mk.index(*k), enc_u32(i as u32));
        out.insert(mk.slot(i as u32), vec![*k]);
    }
    if let Some(h) = header_stored(v.len() as u32, rev) {
        out.insert(mk.header(), h);
    }
}

impl MState {
    pub fn root_bytes(&self) -> Vec<u8> {
        let mut f = BTreeMap::new();
        f.insert(TAG_COUNTER, self.counter.to_le_bytes().to_vec());
        enc_record(&f)
    }

    /// The storage this state occupies, per the spec's key grammar and value encodings.
    pub fn concretize(&self) -> BTreeMap<Vec<u8>, Vec<u8>> {
        let mut out = BTreeMap::new();
        if self.root_written {
            out.insert(root_key(), self.root_bytes());
        }
        concretize_map(&mut out, &flat(), &self.flat, self.flat_rev);
        for (k, v) in &self.look {
            out.insert(look().entry(*k), enc_bytes(v));
        }
        let r = recs();
        for (i, (k, rec)) in self.recs.iter().enumerate() {
            let at = if rec.clearing { rec_flag(*k) } else { r.entry(*k) };
            out.insert(at, rec.bytes());
            out.insert(r.index(*k), enc_u32(i as u32));
            out.insert(r.slot(i as u32), vec![*k]);
            concretize_map(&mut out, &items(*k), &rec.items, rec.items_rev);
        }
        if let Some(h) = header_stored(self.recs.len() as u32, self.recs_rev) {
            out.insert(r.header(), h);
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

/// What the call knows about a record of a map whose value owns storage (D9). The record is
/// at its record key, or (while `remove_step` is clearing it) at its flag key, never both.
#[derive(Clone, Debug, PartialEq, Eq)]
enum ROrig {
    Unknown,
    Absent,
    /// Known present from `storage_has_key`, bytes not read; `bool`: at the flag key.
    PresentUnread(bool),
    Present(Vec<u8>, bool),
}

/// One record's slot. Spec, "Bounded clearing" and API round 2 "Flush": a record being
/// cleared is written to its flag key and its record key removed; a record no longer being
/// cleared is written to its record key and its flag key removed; a delete removes whichever
/// key holds it; the record key's op comes before the flag key's (key order).
#[derive(Clone, Debug)]
pub struct RSlot {
    orig: ROrig,
    /// `None`: unchanged. `Some(None)`: removed. `Some(Some((bytes, clearing)))`.
    pending: Option<Option<(Vec<u8>, bool)>>,
    /// Review F1 (API round 3): a delete happened while the call did not know where (or
    /// whether) the record was stored, so a later write must also remove the other key.
    blind_delete: bool,
}

impl RSlot {
    fn location(&self) -> Option<bool> {
        match self.orig {
            ROrig::PresentUnread(f) | ROrig::Present(_, f) => Some(f),
            _ => None,
        }
    }

    /// The flush's writes and removes for this record, as `(key, Some(bytes) | None)`.
    fn flush_ops(&self, key: &[u8]) -> Vec<(Vec<u8>, Option<Vec<u8>>)> {
        let mut ops = Vec::new();
        let at = self.location();
        let (mut at_key, mut at_flag) = (at == Some(false), at == Some(true));
        if self.blind_delete && self.orig == ROrig::Unknown {
            (at_key, at_flag) = (true, true);
        }
        let same = |b: &[u8]| matches!(&self.orig, ROrig::Present(o, _) if o == b);
        match &self.pending {
            None => {}
            Some(Some((b, true))) => {
                if at_key {
                    ops.push((key.to_vec(), None));
                }
                if !(at_flag && same(b)) {
                    ops.push((flag_of(key), Some(b.clone())));
                }
            }
            Some(Some((b, false))) => {
                if !(at_key && same(b)) {
                    ops.push((key.to_vec(), Some(b.clone())));
                }
                if at_flag {
                    ops.push((flag_of(key), None));
                }
            }
            Some(None) => match self.orig {
                ROrig::Absent => {}
                ROrig::Unknown => {
                    ops.push((key.to_vec(), None));
                    ops.push((flag_of(key), None));
                }
                _ if at_flag => ops.push((flag_of(key), None)),
                _ => ops.push((key.to_vec(), None)),
            },
        }
        ops
    }
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
    /// Records of maps whose values own storage, by record key (D9).
    rslots: BTreeMap<Vec<u8>, RSlot>,
    pub predicted: Predicted,
}

impl CacheModel {
    pub fn new(host: BTreeMap<Vec<u8>, Vec<u8>>) -> Self {
        CacheModel { host, slots: BTreeMap::new(), rslots: BTreeMap::new(), predicted: Predicted::default() }
    }

    fn lookup(&mut self, kind: OpKind, key: &[u8]) -> Option<Vec<u8>> {
        let v = self.host.get(key).cloned();
        let len = if kind == OpKind::Read { v.as_ref().map_or(0, Vec::len) } else { 0 };
        self.predicted.lookups.push((kind, key.to_vec(), len));
        v
    }

    fn rslot(&mut self, key: &[u8]) -> &mut RSlot {
        self.rslots.entry(key.to_vec()).or_insert(RSlot { orig: ROrig::Unknown, pending: None, blind_delete: false })
    }

    /// A record's current `(bytes, clearing)`: 1 read of the record key unless known; a miss
    /// reads the flag key (D9). A record known present from `storage_has_key` is read at the
    /// key that holds it (1 read).
    pub fn rec_read(&mut self, key: &[u8]) -> Option<(Vec<u8>, bool)> {
        let orig = self.rslot(key).orig.clone();
        if let Some(p) = &self.rslot(key).pending {
            return p.clone();
        }
        let flag = flag_of(key);
        let now = match orig {
            ROrig::Present(b, f) => return Some((b, f)),
            ROrig::Absent => return None,
            ROrig::PresentUnread(f) => {
                let b = self.lookup(OpKind::Read, if f { &flag } else { key }).expect("present");
                ROrig::Present(b, f)
            }
            ROrig::Unknown => match self.lookup(OpKind::Read, key) {
                Some(b) => ROrig::Present(b, false),
                None => match self.lookup(OpKind::Read, &flag) {
                    Some(b) => ROrig::Present(b, true),
                    None => ROrig::Absent,
                },
            },
        };
        self.rslot(key).orig = now.clone();
        match now {
            ROrig::Present(b, f) => Some((b, f)),
            _ => None,
        }
    }

    /// `None` if no record is stored, else whether it is being cleared: 1 `storage_has_key` on
    /// the record key unless known; a miss checks the flag key (D9).
    pub fn rec_exists(&mut self, key: &[u8]) -> Option<bool> {
        if let Some(p) = &self.rslot(key).pending {
            return p.as_ref().map(|(_, c)| *c);
        }
        match self.rslot(key).orig.clone() {
            ROrig::Present(_, f) | ROrig::PresentUnread(f) => Some(f),
            ROrig::Absent => None,
            ROrig::Unknown => {
                let now = if self.lookup(OpKind::HasKey, key).is_some() {
                    ROrig::PresentUnread(false)
                } else if self.lookup(OpKind::HasKey, &flag_of(key)).is_some() {
                    ROrig::PresentUnread(true)
                } else {
                    ROrig::Absent
                };
                let r = match now {
                    ROrig::PresentUnread(f) => Some(f),
                    _ => None,
                };
                self.rslot(key).orig = now;
                r
            }
        }
    }

    /// Writes a record that is not being cleared (an insert, or a rewrite of a live record).
    pub fn rec_write(&mut self, key: &[u8], bytes: Vec<u8>) {
        self.rslot(key).pending = Some(Some((bytes, false)));
    }

    /// `remove_step` returned `Pending`: the record (with these bytes) goes to its flag key.
    pub fn rec_set_clearing(&mut self, key: &[u8], bytes: Vec<u8>) {
        self.rslot(key).pending = Some(Some((bytes, true)));
    }

    pub fn rec_delete(&mut self, key: &[u8]) {
        let s = self.rslot(key);
        if s.orig == ROrig::Unknown {
            s.blind_delete = true;
        }
        s.pending = Some(None);
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

    fn host_remove(&mut self, key: &[u8]) {
        let old = self.host.remove(key);
        self.predicted.mutations.push((
            OpKind::Remove,
            key.to_vec(),
            old.as_ref().map_or(0, Vec::len),
            old.map(|o| o.len()),
        ));
    }

    /// Every changed key once, in key order (record and flag keys included), then the root
    /// last (written iff its bytes differ from the bytes this call read for it); then the
    /// cache is empty.
    pub fn flush(&mut self, root: Option<Vec<u8>>) {
        let rk = root_key();
        let slots = core::mem::take(&mut self.slots);
        let rslots = core::mem::take(&mut self.rslots);
        let mut ops: Vec<(Vec<u8>, Option<Vec<u8>>)> = Vec::new();
        for (key, s) in &slots {
            if *key == rk {
                continue;
            }
            match &s.pending {
                None => {}
                Some(Some(b)) => {
                    if s.orig != Orig::Present(b.clone()) {
                        ops.push((key.clone(), Some(b.clone())));
                    }
                }
                Some(None) => {
                    if s.orig != Orig::Absent {
                        ops.push((key.clone(), None));
                    }
                }
            }
        }
        for (key, r) in &rslots {
            assert!(!slots.contains_key(key), "MODEL BUG: a record key used as a plain key");
            ops.extend(r.flush_ops(key));
        }
        ops.sort_by(|a, b| a.0.cmp(&b.0));
        for w in ops.windows(2) {
            assert!(w[0].0 != w[1].0, "MODEL BUG: two flush ops for one key");
        }
        for (key, op) in ops {
            match op {
                Some(b) => self.host_write(&key, b),
                None => self.host_remove(&key),
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
        self.rslots.clear();
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
    /// `try_insert` (T1): first write wins; `Ret::Bool(inserted)`.
    FlatTryInsert(K, Val),
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
    LookTryInsert(K, Val),
    LookGet(K),
    LookContains(K),
    LookGetMut(K, Option<u8>),
    LookEntry(K, Val),
    LookRemove(K),
    LookDiscard(K),
    /// `Map::try_insert` of a record (was `insert_new`): `Ret::Bool(inserted)`; an existing
    /// record (also one being cleared: its index is there) is `Occupied`, not a panic.
    RecTryInsert(K, #[generator(0..3)] u8),
    RecEntryDefault(K),
    RecGetMut(K, #[generator(0..3)] u8),
    RecGet(K),
    RecContains(K),
    RecAtMutInsert(K, K, Val),
    RecAtMutRemove(K, K),
    RecAtLen(K),
    RecRemove(K),
    /// `remove_step(k, n)`: `Ret::Bool(true)` for `Removal::Pending`.
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
    /// `Ok((items, next, revision))` or `Err(CursorGone)`.
    Page(Result<(Vec<(u8, Vec<u8>)>, Option<u8>, u64), ()>),
    KeyPage(Result<(Vec<u8>, Option<u8>, u64), ()>),
    RecPage(Result<(Vec<(u8, u64)>, Option<u8>, u64), ()>),
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

/// Map key-vector operations shared by `flat`, `recs` and every `items` map. `rev` is the
/// map's header revision (T4); `records` says the values own storage (D9: their entries are
/// record slots).
struct KV<'a, V> {
    mk: MapKeys,
    v: &'a mut Vec<(u8, V)>,
    rev: &'a mut u64,
    records: bool,
}

impl<V: Clone> KV<'_, V> {
    fn len(&self, c: &mut CacheModel) -> u32 {
        let h = c.read(&self.mk.header());
        let n = self.v.len() as u32;
        agree("map header", h, header_stored(n, *self.rev));
        n
    }
    /// The header after an insert: length changes, revision kept.
    fn set_len(&self, c: &mut CacheModel, n: u32) {
        match header_stored(n, *self.rev) {
            Some(h) => c.write(&self.mk.header(), h),
            None => c.delete(&self.mk.header()),
        }
    }
    /// The header after a removal (or a batch of removals): the revision goes up by one and
    /// the header is written, also at length 0 (T4: it never resets while the map's owner lives).
    fn set_len_after_removal(&mut self, c: &mut CacheModel, n: u32) {
        *self.rev += 1;
        self.set_len(c, n);
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
    fn delete_entry(&self, c: &mut CacheModel, k: u8) {
        if self.records { c.rec_delete(&self.mk.entry(k)) } else { c.delete(&self.mk.entry(k)) }
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
        self.delete_entry(c, k);
        c.delete(&self.mk.index(k));
        let last = n - 1;
        if i != last {
            let moved = self.slot_key(c, last);
            c.write(&self.mk.slot(i), vec![moved]);
            c.write(&self.mk.index(moved), enc_u32(i));
        }
        c.delete(&self.mk.slot(last));
        self.set_len_after_removal(c, last);
        self.v.swap_remove(i as usize);
    }
    /// Clearing a map whose owner is being removed: header read, then (if not empty) per entry
    /// from the end a slot read and three removes, then the header goes, revision and all.
    fn clear_owned(&mut self, c: &mut CacheModel) {
        let n = self.len(c);
        if n > 0 {
            for i in (0..n).rev() {
                let k = self.slot_key(c, i);
                self.delete_entry(c, k);
                c.delete(&self.mk.index(k));
                c.delete(&self.mk.slot(i));
            }
            self.set_len_after_removal(c, 0);
        }
        c.delete(&self.mk.header());
        self.v.clear();
        *self.rev = 0;
    }
    /// Bounded clear from the end (D9) for a map whose owner is being removed. Returns whether
    /// the map is now empty, in which case its header is gone. When the budget runs out with
    /// entries left, the next slot has already been read. One revision bump per entry.
    fn clear_budgeted(&mut self, c: &mut CacheModel, budget: &mut u32) -> bool {
        loop {
            let n = self.len(c);
            if n == 0 {
                c.delete(&self.mk.header());
                *self.rev = 0;
                return true;
            }
            let k = self.slot_key(c, n - 1);
            if *budget == 0 {
                return false;
            }
            *budget -= 1;
            self.delete_entry(c, k);
            c.delete(&self.mk.index(k));
            c.delete(&self.mk.slot(n - 1));
            self.set_len_after_removal(c, n - 1);
            self.v.pop();
        }
    }
    /// Cursor index (if any), then the header (length and revision).
    fn page_bounds(&self, c: &mut CacheModel, after: Option<u8>, limit: u8) -> Result<(u32, u32, u32, u64), ()> {
        let start = match after {
            None => 0,
            Some(k) => self.index(c, k).ok_or(())? + 1,
        };
        let n = self.len(c);
        let end = start.saturating_add(limit as u32).min(n);
        Ok((start, end, n, *self.rev))
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
    /// T1: the index read decides; an existing key writes nothing and reads no value.
    fn try_insert(&mut self, c: &mut CacheModel, k: u8, val: Vec<u8>) -> bool {
        if self.index(c, k).is_some() {
            return false;
        }
        c.write(&self.mk.entry(k), enc_bytes(&val));
        self.push_key(c, k, val);
        true
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
    #[allow(clippy::type_complexity)]
    fn page(&self, c: &mut CacheModel, after: Option<u8>, limit: u8, keys_only: bool) -> Result<(Vec<(u8, Vec<u8>)>, Option<u8>, u64), ()> {
        let (start, end, n, rev) = self.page_bounds(c, after, limit)?;
        let mut items = Vec::new();
        for i in start..end {
            let k = self.slot_key(c, i);
            let v = if keys_only { Vec::new() } else { self.get(c, k).expect("indexed") };
            items.push((k, v));
        }
        let next = if end < n { items.last().map(|(k, _)| *k) } else { None };
        Ok((items, next, rev))
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

macro_rules! flat_kv {
    ($s:expr) => {
        KV { mk: flat(), v: &mut $s.cur.flat, rev: &mut $s.cur.flat_rev, records: false }
    };
}
macro_rules! recs_kv {
    ($s:expr) => {
        KV { mk: recs(), v: &mut $s.cur.recs, rev: &mut $s.cur.recs_rev, records: true }
    };
}
macro_rules! items_kv {
    ($s:expr, $k:expr, $i:expr) => {{
        let r = &mut $s.cur.recs[$i].1;
        KV { mk: items($k), v: &mut r.items, rev: &mut r.items_rev, records: false }
    }};
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
    /// dedicated harnesses check the panics). Since T1, `try_insert` of an existing record is
    /// `Occupied`, not a panic; only writes to or below a record being cleared panic.
    pub fn would_panic(&self, op: &Op) -> bool {
        let clearing = |k: u8| self.rec_pos(k).is_some_and(|i| self.cur.recs[i].1.clearing);
        match *op {
            Op::RecEntryDefault(K(k)) | Op::RecGetMut(K(k), _) | Op::RecRetyped(K(k), _) => clearing(k),
            Op::RecAtMutInsert(K(k), ..) | Op::RecAtMutRemove(K(k), _) => clearing(k),
            Op::VecSwapRemove(i) => i as usize >= self.cur.vec.len(),
            _ => false,
        }
    }

    /// The record at `k` (1 read, 2 on a miss: the flag key), checked against the abstract
    /// state: its bytes and where they live.
    fn rec_get(&mut self, k: u8) -> Option<usize> {
        let r = self.cache.rec_read(&recs().entry(k));
        let i = self.rec_pos(k);
        let want = i.map(|i| (self.cur.recs[i].1.bytes(), self.cur.recs[i].1.clearing));
        assert_eq!(r, want, "MODEL BUG: cache view and abstract state disagree on record {k}");
        i
    }

    /// `at_mut`'s existence check: 1 `storage_has_key`, 2 on a miss (the flag key).
    fn rec_exists(&mut self, k: u8) -> Option<usize> {
        let r = self.cache.rec_exists(&recs().entry(k));
        let i = self.rec_pos(k);
        assert_eq!(r, i.map(|i| self.cur.recs[i].1.clearing), "MODEL BUG: record {k} existence");
        i
    }

    /// Applies `op` inside the current call. Call-boundary ops are handled by the driver.
    pub fn apply(&mut self, op: &Op) -> Ret {
        match *op {
            Op::EndCall | Op::MidFlush | Op::View(_) | Op::RecRetyped(..) => {
                unreachable!("driver handles call boundaries")
            }
            Op::Counter(x) => {
                self.cur.counter = self.cur.counter.wrapping_add(x as u64);
                Ret::Unit
            }
            Op::FlatInsert(K(k), v) => Ret::Bool(flat_kv!(self).insert(&mut self.cache, k, v.bytes())),
            Op::FlatTryInsert(K(k), v) => Ret::Bool(flat_kv!(self).try_insert(&mut self.cache, k, v.bytes())),
            Op::FlatGet(K(k)) => Ret::Bytes(flat_kv!(self).get(&mut self.cache, k)),
            Op::FlatContains(K(k)) => {
                let e = self.cache.exists(&flat().entry(k));
                assert_eq!(e, pos(&self.cur.flat, k).is_some(), "MODEL BUG: contains");
                Ret::Bool(e)
            }
            Op::FlatGetMut(K(k), x) => {
                let c = &mut self.cache;
                match flat_kv!(self).get(c, k) {
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
                let c = &mut self.cache;
                let mut m = flat_kv!(self);
                match m.get(c, k) {
                    Some(old) => Ret::Bytes(Some(old)),
                    None => {
                        c.write(&flat().entry(k), enc_bytes(&v.bytes()));
                        m.push_key(c, k, v.bytes());
                        Ret::Bytes(Some(v.bytes()))
                    }
                }
            }
            Op::FlatRemove(K(k)) => Ret::Bytes(flat_kv!(self).remove(&mut self.cache, k, true)),
            Op::FlatDiscard(K(k)) => Ret::Bool(flat_kv!(self).remove(&mut self.cache, k, false).is_some()),
            Op::FlatPage(after, limit) => {
                Ret::Page(flat_kv!(self).page(&mut self.cache, after.map(|k| k.0), limit, false))
            }
            Op::FlatKeysPage(after, limit) => Ret::KeyPage(
                flat_kv!(self)
                    .page(&mut self.cache, after.map(|k| k.0), limit, true)
                    .map(|(items, next, rev)| (items.into_iter().map(|(k, _)| k).collect(), next, rev)),
            ),
            Op::FlatIter(skip, take) => {
                let c = &mut self.cache;
                let m = flat_kv!(self);
                let n = m.len(c);
                let mut out = Vec::new();
                for i in (skip as u32)..n.min(skip as u32 + take as u32) {
                    let k = m.slot_key(c, i);
                    out.push((k, m.get(c, k).expect("indexed")));
                }
                Ret::Items(out)
            }
            Op::FlatLen => Ret::U32(flat_kv!(self).len(&mut self.cache)),
            Op::LookInsert(K(k), v) => {
                self.cache.write(&look().entry(k), enc_bytes(&v.bytes()));
                self.cur.look.insert(k, v.bytes());
                Ret::Unit
            }
            Op::LookTryInsert(K(k), v) => {
                // T1, a leaf: 1 `storage_has_key`; absent: 1 write.
                let c = &mut self.cache;
                let e = c.exists(&look().entry(k));
                assert_eq!(e, self.cur.look.contains_key(&k), "MODEL BUG: lookup contains");
                if e {
                    return Ret::Bool(false);
                }
                c.write(&look().entry(k), enc_bytes(&v.bytes()));
                self.cur.look.insert(k, v.bytes());
                Ret::Bool(true)
            }
            Op::LookGet(K(k)) => {
                let r = self.cache.read(&look().entry(k));
                let v = self.cur.look.get(&k).cloned();
                agree("lookup entry", r, v.as_deref().map(enc_bytes));
                Ret::Bytes(v)
            }
            Op::LookContains(K(k)) => {
                let e = self.cache.exists(&look().entry(k));
                assert_eq!(e, self.cur.look.contains_key(&k), "MODEL BUG: lookup contains");
                Ret::Bool(e)
            }
            Op::LookGetMut(K(k), x) => {
                let c = &mut self.cache;
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
                let c = &mut self.cache;
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
                let c = &mut self.cache;
                let r = c.read(&look().entry(k));
                agree("lookup entry", r, self.cur.look.get(&k).map(|v| enc_bytes(v)));
                let old = self.cur.look.remove(&k);
                if old.is_some() {
                    c.delete(&look().entry(k));
                }
                Ret::Bytes(old)
            }
            Op::LookDiscard(K(k)) => {
                self.cache.delete(&look().entry(k));
                self.cur.look.remove(&k);
                Ret::Unit
            }
            Op::RecTryInsert(K(k), n) => {
                // T1: `Map::try_insert` = `insert`: the index read decides, so a record being
                // cleared (its index is still there) is `Occupied` too. The record key is
                // written blind.
                let c = &mut self.cache;
                let mut m = recs_kv!(self);
                if m.index(c, k).is_some() {
                    return Ret::Bool(false);
                }
                let rec = RecM::new(n as u64);
                c.rec_write(&recs().entry(k), rec.bytes());
                m.push_key(c, k, rec);
                Ret::Bool(true)
            }
            Op::RecEntryDefault(K(k)) => match self.rec_get(k) {
                Some(i) => Ret::U64(self.cur.recs[i].1.n()),
                None => {
                    // the record and flag keys both missed; no index read
                    let rec = RecM::new(0);
                    let c = &mut self.cache;
                    c.rec_write(&recs().entry(k), rec.bytes());
                    recs_kv!(self).push_key(c, k, rec);
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
                    self.cache.rec_write(&recs().entry(k), b);
                    Ret::OptU64(Some(n))
                }
            },
            Op::RecGet(K(k)) => {
                let i = self.rec_get(k);
                Ret::OptU64(i.map(|i| self.cur.recs[i].1.n()))
            }
            Op::RecContains(K(k)) => Ret::Bool(self.rec_exists(k).is_some()),
            Op::RecAtMutInsert(K(k), K(k2), v) => {
                // `at_mut`: an existence check, no read, no decode; `None` if absent.
                let Some(i) = self.rec_exists(k) else { return Ret::OptU64(None) };
                let new = items_kv!(self, k, i).insert(&mut self.cache, k2, v.bytes());
                Ret::Bool(new)
            }
            Op::RecAtMutRemove(K(k), K(k2)) => {
                let Some(i) = self.rec_exists(k) else { return Ret::OptU64(None) };
                Ret::Bytes(items_kv!(self, k, i).remove(&mut self.cache, k2, true))
            }
            // `at(&k)` reads nothing; `.items.len()` reads the header (absent record: 0).
            Op::RecAtLen(K(k)) => Ret::U32(self.rec_items_len(k)),
            Op::RecRemove(K(k)) => {
                // also allowed on a record being cleared: removes it (from its flag key) whole
                let Some(i) = self.rec_get(k) else { return Ret::OptU64(None) };
                let n = self.cur.recs[i].1.n();
                items_kv!(self, k, i).clear_owned(&mut self.cache);
                recs_kv!(self).unlink(&mut self.cache, k);
                Ret::OptU64(Some(n))
            }
            Op::RecRemoveSome(K(k), n) => {
                let Some(i) = self.rec_get(k) else { return Ret::Bool(false) };
                let mut budget = n as u32;
                let done = items_kv!(self, k, i).clear_budgeted(&mut self.cache, &mut budget);
                if done {
                    // `remove_inner(k, false)`: clear_children again (the header, already
                    // gone), then unlink.
                    items_kv!(self, k, i).clear_owned(&mut self.cache);
                    recs_kv!(self).unlink(&mut self.cache, k);
                    Ret::Bool(false)
                } else {
                    // `Pending`: the record moves to its flag key (unchanged bytes, F3)
                    let r = &mut self.cur.recs[i].1;
                    r.clearing = true;
                    let b = r.bytes();
                    self.cache.rec_set_clearing(&recs().entry(k), b);
                    Ret::Bool(true)
                }
            }
            Op::RecPage(after, limit) => {
                let c = &mut self.cache;
                let m = recs_kv!(self);
                let r = m.page_bounds(c, after.map(|k| k.0), limit).map(|(start, end, n, rev)| {
                    let mut out = Vec::new();
                    for i in start..end {
                        let k = m.slot_key(c, i);
                        let rb = c.rec_read(&recs().entry(k));
                        let rec = &m.v[i as usize].1;
                        assert_eq!(rb, Some((rec.bytes(), rec.clearing)), "MODEL BUG: paged record");
                        out.push((k, rec.n()));
                    }
                    let next = if end < n { out.last().map(|(k, _)| *k) } else { None };
                    (out, next, rev)
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
                // `Vector` keeps the bare `u32` header, removed at 0 (no T4 revision)
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
        let a = flat_kv!(self).get(&mut self.cache, k);
        let r = self.cache.read(&look().entry(k));
        let b = self.cur.look.get(&k).cloned();
        agree("lookup entry", r, b.as_deref().map(enc_bytes));
        let n = self.rec_get(k).map(|i| self.cur.recs[i].1.n());
        let len = self.rec_items_len(k);
        (Ret::View(a, b, n, len), self.end_view())
    }

    fn rec_items_len(&mut self, k: u8) -> u32 {
        let i = self.rec_pos(k);
        let (mut empty, mut zero) = (Vec::new(), 0u64);
        let (v, rev) = match i {
            Some(i) => {
                let r = &mut self.cur.recs[i].1;
                (&mut r.items, &mut r.items_rev)
            }
            None => (&mut empty, &mut zero),
        };
        KV { mk: items(k), v, rev, records: false }.len(&mut self.cache)
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
                self.cache.rec_write(&recs().entry(k), b);
                Ret::OptU64(Some(note as u64))
            }
        };
        (ret, self.end_call())
    }
}
