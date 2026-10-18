//! Guarantee 1, Addressable: a key is computed from schema + path without reads, and two
//! distinct paths in one schema never share a key (the collision fix). Also F1, encoder
//! boundaries: zero-byte keys, every table kind, varint transitions through the full `u32`
//! range, paths at the 2,048-byte limit, colliding hash prefixes.
//!
//! Paths are generated from the schema grammar, not as free segment soup (field tag 1 and kind
//! `INDEX` are both byte `01`; the argument relies on the schema fixing which segment comes
//! next). A generated pair is kept only if it is *schema-consistent*: wherever the two paths
//! have taken the same field tags so far, the field has the same collection type and key type.
//! Every key is built by the PRODUCTION builder (`key::Path`).

use crate::state::key::{DEFAULT_ROOT, ENTRY_HASH_LEN, MAX_KEY_LEN, Path, h16, kind};
use crate::state::model::spec_key::{self, Seg};
use bolero::generator::TypeGenerator;

/// Borsh shape of a map key type: fixed width `0..=8` (`()`, integers, `[u8; N]`), or a
/// length-prefixed byte string (`Vec<u8>` / `String`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, TypeGenerator)]
pub enum KeyTy {
    Fixed(#[generator(0..=8)] u8),
    Var,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, TypeGenerator)]
pub enum Coll {
    Lookup(KeyTy),
    Map(KeyTy),
    Vector,
    Lazy,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, TypeGenerator)]
pub enum Access {
    /// The value position: a map entry, or a `Lazy`'s own key.
    Entry,
    Index,
    Slot(u32),
    Header,
}

/// One hop: a field tag, then the collection's table and key.
#[derive(Clone, Copy, Debug, TypeGenerator)]
pub struct Hop {
    #[generator(1..16384)]
    pub tag: u32,
    pub coll: Coll,
    pub access: Access,
    pub key: [u8; 8],
    #[generator(0..=8)]
    pub klen: u8,
    /// Placement hash used when the hash is symbolic (Kani); the random engine uses sha256.
    pub hash: [u8; ENTRY_HASH_LEN],
}

pub const DEPTH: usize = 4;

#[derive(Clone, Copy, Debug, TypeGenerator)]
pub struct SPath {
    pub hops: [Hop; DEPTH],
    #[generator(0..=DEPTH as u8)]
    pub depth: u8,
}

impl Hop {
    fn wf(&self) -> bool {
        match (self.coll, self.access) {
            (Coll::Lookup(_), Access::Entry) => true,
            (Coll::Lookup(_), _) => false,
            (Coll::Map(_), _) => true,
            (Coll::Vector, Access::Slot(_) | Access::Header) => true,
            (Coll::Vector, _) => false,
            (Coll::Lazy, Access::Entry) => true,
            (Coll::Lazy, _) => false,
        }
    }
    /// Whether a value lives here that can own further storage (so the path may continue).
    fn value_position(&self) -> bool {
        matches!((self.coll, self.access), (Coll::Lookup(_) | Coll::Map(_) | Coll::Lazy, Access::Entry) | (Coll::Vector, Access::Slot(_)))
    }
    fn key_ty(&self) -> Option<KeyTy> {
        match self.coll {
            Coll::Lookup(t) | Coll::Map(t) => Some(t),
            _ => None,
        }
    }
    /// `borsh(k)` for the hop's key type.
    fn enc(&self) -> std::vec::Vec<u8> {
        match self.key_ty() {
            Some(KeyTy::Fixed(w)) => self.key[..w as usize].to_vec(),
            Some(KeyTy::Var) => {
                let mut v = (self.klen as u32).to_le_bytes().to_vec();
                v.extend_from_slice(&self.key[..self.klen as usize]);
                v
            }
            None => std::vec::Vec::new(),
        }
    }
    fn has_key(&self) -> bool {
        matches!(self.access, Access::Entry | Access::Index) && self.key_ty().is_some()
    }
    /// Abstract identity of the hop (the hash is not part of it).
    fn same(&self, o: &Hop) -> bool {
        self.tag == o.tag
            && self.coll == o.coll
            && self.access == o.access
            && (!self.has_key() || self.enc() == o.enc())
    }
}

impl SPath {
    fn hops(&self) -> &[Hop] {
        &self.hops[..self.depth as usize]
    }
    pub fn wf(&self) -> bool {
        let h = self.hops();
        h.iter().all(Hop::wf) && h.iter().rev().skip(1).all(Hop::value_position)
    }
    fn same(&self, o: &SPath) -> bool {
        self.depth == o.depth && self.hops().iter().zip(o.hops()).all(|(a, b)| a.same(b))
    }
    /// `self` is a proper ancestor of `o`: every hop of `self` is a hop of `o`.
    fn ancestor_of(&self, o: &SPath) -> bool {
        self.depth < o.depth && self.hops().iter().zip(o.hops()).all(|(a, b)| a.same(b))
    }

    /// Encodes with the production builder. `real_hash`: sha256 (`entry_raw`), else the hop's
    /// symbolic hash (`entry_hashed`).
    pub fn encode(&self, real_hash: bool) -> std::vec::Vec<u8> {
        let mut p = Path::root(DEFAULT_ROOT);
        for h in self.hops() {
            p = p.field(h.tag);
            let entry = |p: Path| if real_hash { p.entry_raw(&h.enc()) } else { p.entry_hashed(&h.hash, &h.enc()) };
            p = match (h.coll, h.access) {
                (Coll::Lazy, _) => p,
                (_, Access::Entry) => entry(p.kind(kind::ENTRIES)),
                (_, Access::Index) => entry(p.kind(kind::INDEX)),
                (Coll::Vector, Access::Slot(i)) => p.kind(kind::ENTRIES).slot(i),
                (_, Access::Slot(i)) => p.kind(kind::SLOTS).slot(i),
                (_, Access::Header) => p.kind(kind::HEADER),
            };
        }
        p.as_bytes().to_vec()
    }

    /// The same path through the model's independent spec encoder (real sha256).
    pub fn spec_segs(&self) -> std::vec::Vec<Seg> {
        let mut s = std::vec::Vec::new();
        for h in self.hops() {
            s.push(Seg::Field(h.tag));
            match (h.coll, h.access) {
                (Coll::Lazy, _) => {}
                (_, Access::Entry) => s.extend([Seg::Kind(spec_key::ENTRIES), Seg::Entry(h.enc())]),
                (_, Access::Index) => s.extend([Seg::Kind(spec_key::INDEX), Seg::Entry(h.enc())]),
                (Coll::Vector, Access::Slot(i)) => s.extend([Seg::Kind(spec_key::ENTRIES), Seg::Slot(i)]),
                (_, Access::Slot(i)) => s.extend([Seg::Kind(spec_key::SLOTS), Seg::Slot(i)]),
                (_, Access::Header) => s.push(Seg::Kind(spec_key::HEADER)),
            }
        }
        s
    }
}

/// Both paths live in one schema: wherever they have taken the same tags so far, the field
/// has the same collection and key type.
fn consistent(a: &SPath, b: &SPath) -> bool {
    let n = a.depth.min(b.depth) as usize;
    for d in 0..n {
        if (0..=d).all(|i| a.hops[i].tag == b.hops[i].tag) && a.hops[d].coll != b.hops[d].coll {
            return false;
        }
    }
    true
}

/// The property. Equal bytes imply the same path; one key a prefix of another implies an
/// ancestor (a record and the storage it owns).
fn check_pair(a: &SPath, b: &SPath, real_hash: bool) {
    if !a.wf() || !b.wf() || !consistent(a, b) {
        return;
    }
    let (ka, kb) = (a.encode(real_hash), b.encode(real_hash));
    if ka == kb {
        assert!(a.same(b), "two distinct paths share a key: {a:?} / {b:?}");
    }
    if kb.starts_with(&ka) {
        assert!(a.same(b) || a.ancestor_of(b), "a key is a prefix of a non-descendant's key: {a:?} / {b:?}");
    }
    if ka.starts_with(&kb) {
        assert!(a.same(b) || b.ancestor_of(a), "a key is a prefix of a non-descendant's key: {b:?} / {a:?}");
    }
}

/// Random engine only: draw tags from LEB128 edge values (and values 128 / 256 apart), so
/// near-collisions that a uniform draw would almost never produce do occur. Kani needs no
/// help: its tags are symbolic.
fn edge_tags(p: &SPath) -> SPath {
    const PICK: [u32; 10] = [1, 2, 127, 128, 129, 255, 256, 257, 16_383, 3];
    let mut p = *p;
    for h in &mut p.hops {
        h.tag = PICK[h.tag as usize % PICK.len()];
    }
    p
}

/// Copies the first `share` hops of `a` into `b`, so the random engine meets near-collisions
/// (Kani covers every pair regardless).
fn share_prefix(a: &SPath, b: &SPath, share: u8) -> SPath {
    if !cfg!(kani) && share >= 0x80 {
        // the same path with one hop's tag replaced: the shape a tag-encoding bug collides on
        let mut c = *a;
        let i = (share as usize & 0x3) % DEPTH;
        c.hops[i].tag = b.hops[i].tag;
        return c;
    }
    let mut b = *b;
    let n = (share as usize).min(a.depth as usize).min(DEPTH);
    b.hops[..n].copy_from_slice(&a.hops[..n]);
    b
}

/// G1 core claim, model-checked. Bound (Kani): two symbolic paths of depth <= 4, tags in
/// 1..2^14 (one- and two-byte LEB128), map keys of width 0..=8 or length-prefixed with
/// 0..=8 bytes, every collection and table kind, and the 16-byte placement hash as FREE
/// symbolic bytes per hop (so the claim holds even if sha256 collides). Random engine: the
/// same harness with the real sha256, 20,000 pairs.
#[test]
#[cfg_attr(kani, kani::proof)]
#[cfg_attr(kani, kani::unwind(40))]
#[cfg_attr(kani, kani::solver(cadical))]
fn g1_addressable_distinct_paths_distinct_keys() {
    bolero::check!().with_type::<(SPath, SPath, u8)>().with_iterations(20_000).for_each(|(a, b, share)| {
        let (a, b) = if cfg!(kani) { (*a, *b) } else { (edge_tags(a), edge_tags(b)) };
        let b = share_prefix(&a, &b, *share);
        check_pair(&a, &b, !cfg!(kani));
    });
}

/// F1: field tags across the whole `u32` range. Model-checked for every pair of tags
/// (Kani, unwind 6 covers the 5-byte LEB128): the production field hop is
/// `root ++ LEB128(tag)`, it decodes back with the record varint reader, and two different
/// tags give byte strings neither of which is a prefix of the other.
#[test]
#[cfg_attr(kani, kani::proof)]
#[cfg_attr(kani, kani::unwind(7))]
fn g1_field_tags_are_prefix_free_over_u32() {
    bolero::check!().with_type::<(u32, u32)>().with_iterations(20_000).for_each(|(t1, t2)| {
        let (t1, t2) = ((*t1).max(1), (*t2).max(1));
        let a = Path::root(DEFAULT_ROOT).field(t1);
        let b = Path::root(DEFAULT_ROOT).field(t2);
        let mut pos = 1;
        assert_eq!(crate::state::record::read_varint_u32(a.as_bytes(), &mut pos), Ok(t1));
        assert_eq!(pos, a.len());
        if t1 != t2 {
            assert!(!b.as_bytes().starts_with(a.as_bytes()) && !a.as_bytes().starts_with(b.as_bytes()));
        }
    });
}

/// F1: varint length transitions, explicitly, against the spec encoder.
#[test]
fn g1_field_tag_varint_transitions() {
    for t in [1u32, 127, 128, 16_383, 16_384, (1 << 21) - 1, 1 << 21, (1 << 28) - 1, 1 << 28, u32::MAX] {
        assert_eq!(Path::root(DEFAULT_ROOT).field(t).as_bytes(), spec_key::encode(&[Seg::Field(t)]), "tag {t}");
    }
}

/// G1 against the model: the production key equals the spec encoder's key for every
/// generated path (real sha256). Random engine, 20,000 paths.
#[test]
fn g1_production_keys_equal_spec_keys() {
    bolero::check!().with_type::<SPath>().with_iterations(20_000).for_each(|a| {
        if a.wf() {
            assert_eq!(a.encode(true), spec_key::encode(&a.spec_segs()), "{a:?}");
        }
    });
}

/// A long path: up to 12 hops, any `u32` tag, keys up to 512 bytes, to reach the 2,048-byte
/// limit.
#[derive(Clone, Debug, TypeGenerator)]
pub struct LongHop {
    pub tag: u32,
    pub coll: Coll,
    pub access: Access,
    #[generator(bolero::produce::<std::vec::Vec<u8>>().with().len(0usize..=512))]
    pub key: std::vec::Vec<u8>,
}

fn long_segs(hops: &[LongHop]) -> Option<std::vec::Vec<Seg>> {
    let mut s = std::vec::Vec::new();
    for (i, h) in hops.iter().enumerate() {
        let probe = Hop { tag: h.tag.max(1), coll: h.coll, access: h.access, key: [0; 8], klen: 0, hash: [0; ENTRY_HASH_LEN] };
        if !probe.wf() || (i + 1 < hops.len() && !probe.value_position()) {
            return None;
        }
        let enc = match probe.key_ty() {
            Some(KeyTy::Fixed(w)) => h.key.iter().copied().chain(core::iter::repeat(0)).take(w as usize).collect(),
            Some(KeyTy::Var) => {
                let mut v = (h.key.len() as u32).to_le_bytes().to_vec();
                v.extend_from_slice(&h.key);
                v
            }
            None => std::vec::Vec::new(),
        };
        s.push(Seg::Field(h.tag.max(1)));
        match (h.coll, h.access) {
            (Coll::Lazy, _) => {}
            (_, Access::Entry) => s.extend([Seg::Kind(spec_key::ENTRIES), Seg::Entry(enc)]),
            (_, Access::Index) => s.extend([Seg::Kind(spec_key::INDEX), Seg::Entry(enc)]),
            (Coll::Vector, Access::Slot(i)) => s.extend([Seg::Kind(spec_key::ENTRIES), Seg::Slot(i)]),
            (_, Access::Slot(i)) => s.extend([Seg::Kind(spec_key::SLOTS), Seg::Slot(i)]),
            (_, Access::Header) => s.push(Seg::Kind(spec_key::HEADER)),
        }
    }
    Some(s)
}

fn build(segs: &[Seg]) -> std::vec::Vec<u8> {
    let mut p = Path::root(DEFAULT_ROOT);
    for s in segs {
        p = match s {
            Seg::Field(t) => p.field(*t),
            Seg::Kind(k) => p.kind(*k),
            Seg::Entry(e) => p.entry_raw(e),
            Seg::Slot(i) => p.slot(*i),
        };
    }
    p.as_bytes().to_vec()
}

/// F1 + G1 on long paths (random engine only, 5,000 paths): any `u32` tag, keys up to 512
/// bytes, up to 12 hops. Every path whose spec encoding fits in 2,048 bytes is built by
/// production with exactly those bytes; a path over the limit makes production panic.
#[test]
fn g1_long_paths_up_to_the_protocol_limit() {
    let g = bolero::produce::<std::vec::Vec<LongHop>>().with().len(0usize..=12);
    bolero::check!().with_generator(g).with_iterations(5_000).for_each(|hops| {
        let Some(segs) = long_segs(hops) else { return };
        let want = spec_key::encode(&segs);
        if want.len() <= MAX_KEY_LEN {
            assert_eq!(build(&segs), want);
        } else {
            let r = std::panic::catch_unwind(|| build(&segs));
            assert!(r.is_err(), "a {}-byte key was accepted", want.len());
        }
    });
}

/// F1: exactly at the limit. A `Vec<u8>`-keyed entry under one field: root (1) + tag (1) +
/// kind (1) + hash (16) + 4-byte length + n bytes. n = 2,025 lands on 2,048 bytes.
#[test]
fn g1_key_of_exactly_2048_bytes_is_accepted_2049_panics() {
    let base = Path::root(DEFAULT_ROOT).field(1).kind(kind::ENTRIES);
    let fixed = 1 + 1 + 1 + ENTRY_HASH_LEN + 4;
    let n = MAX_KEY_LEN - fixed;
    let mut enc = (n as u32).to_le_bytes().to_vec();
    enc.extend(core::iter::repeat_n(7u8, n));
    assert_eq!(base.entry_raw(&enc).len(), MAX_KEY_LEN);
    let mut enc2 = ((n + 1) as u32).to_le_bytes().to_vec();
    enc2.extend(core::iter::repeat_n(7u8, n + 1));
    assert!(std::panic::catch_unwind(|| base.entry_raw(&enc2)).is_err());
}

/// F1: colliding hash prefixes. Two different keys forced to the SAME placement hash still
/// give different keys (Borsh is the uniqueness, the hash only places), including the
/// zero-byte key `()` against a one-byte key, and a length-prefixed key against a longer one.
#[test]
fn g1_identical_hashes_still_give_distinct_keys() {
    let base = Path::root(DEFAULT_ROOT).field(3).kind(kind::ENTRIES);
    let h = [0xAB; ENTRY_HASH_LEN];
    let pairs: &[(&[u8], &[u8])] = &[(&[], &[0]), (&[1, 0, 0, 0, 9], &[2, 0, 0, 0, 9, 9]), (&[5], &[6])];
    for (a, b) in pairs {
        assert_ne!(base.entry_hashed(&h, a), base.entry_hashed(&h, b));
    }
    // and h16 really is sha256[..ENTRY_HASH_LEN] of borsh(k)
    assert_eq!(base.entry_raw(&[5]).as_bytes()[3..3 + ENTRY_HASH_LEN], h16(&[5]));
}

/// G1, "without reading state": after any seeded history, `recs.at(&k).items.get(&k2)` makes
/// exactly ONE host call, a read of the key the model's spec encoder computes for that path;
/// the parent record is never read. Computing the key itself makes no host call. Random
/// engine, 2,000 cases, seeds of up to 12 items over 5 x 5 keys.
#[test]
fn g1_nested_get_reads_only_its_own_key() {
    use super::fixture::{MRoot, Rec};
    use crate::state::model::{self, K, Val};
    use crate::state::testing;
    use crate::state::{flush_root, load_root};
    let g = bolero::produce::<(K, K, std::vec::Vec<(K, K, Val)>)>();
    bolero::check!().with_generator(g).with_iterations(2_000).for_each(|(k, k2, seed)| {
        testing::use_mock();
        let mut root = load_root::<MRoot>();
        for (a, b, v) in seed.iter().take(12) {
            root.recs.entry(a.0).or_insert_with(Rec::default).items.insert(b.0, v.bytes());
        }
        flush_root(&root);
        drop(root);
        testing::begin_call();
        let root = load_root::<MRoot>();
        let before = testing::receipt().ops.len();
        let path = Path::root(DEFAULT_ROOT).field(4).kind(kind::ENTRIES).entry(&k.0).field(2).kind(kind::ENTRIES).entry(&k2.0);
        assert_eq!(testing::receipt().ops.len(), before, "computing a key made a host call");
        assert_eq!(path.as_bytes(), model::items(k.0).entry(k2.0));
        let got = root.recs.at(&k.0).items.get(&k2.0).cloned();
        let r = testing::receipt();
        assert_eq!(r.ops.len(), before + 1);
        assert_eq!(r.ops[before].kind, testing::OpKind::Read);
        assert_eq!(r.ops[before].key, model::items(k.0).entry(k2.0));
        let want = seed.iter().take(12).rev().find(|(a, b, _)| *a == *k && *b == *k2).map(|(_, _, v)| v.bytes());
        assert_eq!(got, want);
        drop(root);
    });
}
