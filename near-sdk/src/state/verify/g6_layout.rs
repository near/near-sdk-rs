//! Guarantee 6, Permanent layout (the record-codec kernel). A deploy never moves or rewrites
//! existing data by itself: decoding is a pure function of the stored bytes (no host call can
//! happen in it), missing tags default (or fail naming the tag if `required`), unknown tags are
//! preserved through a read-modify-write, reserved tags are dropped, and every record has
//! exactly one encoding. Drives the production codec (`record::RecordWriter`, `record::parse`)
//! and the helpers the derive expands to (`root::__private::{field, field_or,
//! field_required}`). The derive's own output is checked in `tests/state_v2_verify.rs`.
//! Since API round 5 a collection field leaves a kind marker in its record; since Verification
//! round 2 every field key carries a wire type (`tag << 2 | wire`: 0 scalar, 1 marker, 2 nested
//! record), so a marker is unambiguous. The identity and canonical harnesses draw the wire type
//! symbolically; `g6_kind_markers_are_wire_typed_preserved_and_checked_on_removal` covers the
//! removal check.

use crate::state::key::{DEFAULT_ROOT, Path};
use crate::state::record::{self, RecordError, RecordWriter, Wire};
use crate::state::root::__private as p;
use bolero::generator::TypeGenerator;

#[derive(Clone, Copy, Debug, TypeGenerator)]
pub struct Field {
    #[generator(1..300)]
    pub tag: u32,
    pub body: [u8; 16],
    #[generator(0..=16)]
    pub len: u8,
    pub present: bool,
    /// The wire type, `wire % 3`: 0 scalar, 1 kind marker (the body is cut to 1 byte), 2
    /// nested record (opaque bytes to the codec).
    pub wire: u8,
}

/// Longest byte string the canonical-form harness tries: 4 under Kani, 64 otherwise.
const CAN_MAX: usize = if cfg!(kani) { 4 } else { 64 };

const NF: usize = if cfg!(kani) { 2 } else { 4 };

fn wire_of(w: u8) -> Wire {
    match w % 3 {
        0 => Wire::Scalar,
        1 => Wire::Marker,
        _ => Wire::Record,
    }
}

fn distinct(fields: &[Field]) -> Option<std::vec::Vec<(u32, Wire, std::vec::Vec<u8>)>> {
    let mut v: std::vec::Vec<(u32, Wire, std::vec::Vec<u8>)> = std::vec::Vec::new();
    for f in fields.iter().filter(|f| f.present) {
        if v.iter().any(|(t, _, _)| *t == f.tag) {
            return None;
        }
        let wire = wire_of(f.wire);
        let len = if wire == Wire::Marker { 1 } else { f.len as usize };
        v.push((f.tag, wire, f.body[..len].to_vec()));
    }
    Some(v)
}

/// Encode then parse is the identity (fields come back sorted by tag, each with its wire type),
/// whatever order they were pushed in. Bound (Kani): up to 2 fields, tags in 1..300 (one- and
/// two-byte field keys: one byte holds tags 1..=31), any of the 3 wire types, bodies of 0..=4
/// bytes (a marker 1). NOT VERIFIED in the Soundness round: the only bound attempted, 2 fields x
/// 4 bytes (wire types not yet in the format), did not finish in 20 min (Kani 0.68, CPU shared
/// with cargo-mutants). Random engine: up to 4 fields, bodies 0..=16, 20,000 records.
#[test]
#[cfg_attr(kani, kani::proof)]
#[cfg_attr(kani, kani::unwind(18))]
fn g6_record_encode_then_parse_is_identity() {
    bolero::check!().with_type::<[Field; NF]>().with_iterations(20_000).for_each(|fields| {
        if cfg!(kani) && fields.iter().any(|f| f.len > 4) {
            return;
        }
        let Some(v) = distinct(fields) else { return };
        let mut w = RecordWriter::new();
        for (t, wire, b) in &v {
            w.push_wire(*t, *wire, b.clone());
        }
        let enc = w.finish(None, |_| true);
        let mut want = v.clone();
        want.sort_by_key(|(t, _, _)| *t);
        let got = record::parse(&enc).expect("own encoding parses");
        assert_eq!(got.len(), want.len());
        for ((gt, gw, gb), (wt, ww, wb)) in got.iter().zip(&want) {
            assert_eq!((*gt, *gw, *gb), (*wt, *ww, &wb[..]));
        }
    });
}

/// Canonical form: any byte string that parses re-encodes to exactly itself, so a record has
/// one encoding. (Since API round 2 the D9 clearing flag is a separate key, not a leading
/// `00 00` in the record, and tag 0 is rejected, so there is no prefix to skip. Since
/// Verification round 2 each field carries its wire type, re-encoded as parsed; wire 3 and a
/// marker of length other than 1 are rejected.) Bound (Kani): every byte string of length
/// 0..=4. NOT VERIFIED in the Soundness round: 6 bytes (about 27 min), 4 bytes as a symbolic
/// Vec (15 min) and 4 bytes as a fixed array (20 min) did not finish. Random engine: 50,000
/// strings of up to 64 bytes.
#[test]
#[cfg_attr(kani, kani::proof)]
#[cfg_attr(kani, kani::unwind(7))]
fn g6_record_parse_accepts_only_canonical_encodings() {
    // a fixed array plus a symbolic length: bolero's Vec generator is costly under Kani
    bolero::check!().with_type::<([u8; CAN_MAX], u8)>().with_iterations(50_000).for_each(|(arr, n)| {
        let b = &arr[..*n as usize % (CAN_MAX + 1)];
        if let Ok(fields) = record::parse(b) {
            let mut w = RecordWriter::new();
            for (t, wire, body) in &fields {
                assert!(*wire != Wire::Marker || body.len() == 1, "a marker is one byte");
                w.push_wire(*t, *wire, body.to_vec());
            }
            let enc = w.finish(None, |_| true);
            assert_eq!(&enc[..], b, "two encodings of one record");
        }
    });
}

/// How a later schema declares a tag.
#[derive(Clone, Copy, Debug, PartialEq, Eq, TypeGenerator)]
pub enum Decl {
    /// Not a field (an unknown tag to this schema).
    Unknown,
    /// `#[key(n)] f: u8` (missing decodes as `Default`, 0).
    Default,
    /// `#[key(n, default = 9)]`.
    DefaultExpr,
    /// `#[key(n, required)]`.
    Required,
    /// Listed in `reserved(n)`.
    Reserved,
}

/// Tags in the schema-evolution harness: 4 (random engine), 2 (Kani; 4 did not finish
/// in 15 min, 3 did not finish in 30 min).
const TAGS: usize = if cfg!(kani) { 2 } else { 4 };

/// Schema evolution, the kernel the derive expands to. A record written by schema S1 (any
/// subset of tags 1..=4, `u8` fields) is read by schema S2 (each tag Unknown, Default,
/// DefaultExpr, Required or Reserved): present tags decode to their value, missing ones to
/// their default, a missing required tag fails with `Missing { tag }` naming it. S2 then
/// rewrites the record (every known value + 1) merging the original bytes: unknown tags keep
/// their exact bytes, reserved tags are dropped, known tags carry the new values. Reading the
/// rewrite back with S1 (a rollback) sees its own unknown-to-S2 fields intact. Bound (Kani):
/// all 2^2 written subsets x 5^2 schemas x u8 values (tags 1..=2). NOT VERIFIED: 4, 3 and 2
/// tags each did not finish within 15-30 min. Random engine: 20,000 cases.
#[test]
#[cfg_attr(kani, kani::proof)]
#[cfg_attr(kani, kani::unwind(10))]
fn g6_schema_evolution_defaults_required_unknown_reserved() {
    let g = bolero::produce::<([Option<u8>; TAGS], [Decl; TAGS])>();
    bolero::check!().with_generator(g).with_iterations(20_000).for_each(|(s1, s2)| {
        let path = Path::root(DEFAULT_ROOT).field(9);
        // S1 writes
        let mut w = RecordWriter::new();
        for (i, v) in s1.iter().enumerate() {
            if let Some(v) = v {
                w.push(i as u32 + 1, vec![*v]);
            }
        }
        let stored = w.finish(None, |_| true);
        // S2 reads
        let fields = p::parse(&stored).expect("parses");
        let mut decoded = [None::<u8>; TAGS];
        let mut failed = None;
        for (i, d) in s2.iter().enumerate() {
            let tag = i as u32 + 1;
            let r: Option<Result<u8, RecordError>> = match d {
                Decl::Default => Some(p::field::<u8>(&fields, tag, Some(&path))),
                Decl::DefaultExpr => Some(p::field_or::<u8>(&fields, tag, Some(&path), || 9)),
                Decl::Required => Some(p::field_required::<u8>(&fields, tag, Some(&path))),
                Decl::Unknown | Decl::Reserved => None,
            };
            match r {
                None => {}
                Some(Ok(v)) => {
                    let want = match (s1[i], d) {
                        (Some(x), _) => x,
                        (None, Decl::DefaultExpr) => 9,
                        (None, _) => 0,
                    };
                    assert_eq!(v, want, "tag {tag}");
                    decoded[i] = Some(v);
                }
                Some(Err(e)) => {
                    assert_eq!((*d, s1[i]), (Decl::Required, None), "unexpected error {e:?}");
                    assert_eq!(e, RecordError::Missing { tag });
                    failed.get_or_insert(tag);
                }
            }
        }
        if failed.is_some() {
            return; // the record does not decode under S2; nothing is rewritten
        }
        // S2 rewrites (read-modify-write: merges the original bytes)
        let known = |t: u32| matches!(s2[t as usize - 1], Decl::Default | Decl::DefaultExpr | Decl::Required);
        let mut w = RecordWriter::new();
        for (i, v) in decoded.iter().enumerate() {
            if let Some(v) = v {
                w.push(i as u32 + 1, vec![v.wrapping_add(1)]);
            }
        }
        let rewritten = w.finish(Some(&stored), |t| t as usize <= TAGS && !known(t) && s2[t as usize - 1] != Decl::Reserved);
        let after = record::parse(&rewritten).expect("parses");
        for i in 0..TAGS {
            let tag = i as u32 + 1;
            let got = record::find(&after, tag);
            match s2[i] {
                Decl::Unknown => assert_eq!(got, s1[i].as_ref().map(core::slice::from_ref), "unknown tag {tag} not preserved"),
                Decl::Reserved => assert_eq!(got, None, "reserved tag {tag} not dropped"),
                _ => assert_eq!(got, Some(&[decoded[i].unwrap().wrapping_add(1)][..])),
            }
        }
    });
}

/// Random engine only (20,000 cases): any `u32` tag (field keys up to 5 bytes) and bodies up
/// to 300 bytes, so the length varint crosses 127/128; encode then parse is the identity.
#[test]
fn g6_record_roundtrip_large_tags_and_bodies() {
    let g = bolero::produce::<std::vec::Vec<(u32, std::vec::Vec<u8>)>>().with().len(0usize..=6);
    bolero::check!().with_generator(g).with_iterations(20_000).for_each(|fs| {
        let mut v: std::vec::Vec<(u32, std::vec::Vec<u8>)> = std::vec::Vec::new();
        for (t, b) in fs {
            let t = (*t).max(1);
            if !v.iter().any(|(x, _)| *x == t) {
                v.push((t, b.iter().copied().cycle().take(b.len() * 50 % 301).collect()));
            }
        }
        let mut w = RecordWriter::new();
        for (t, b) in &v {
            w.push(*t, b.clone());
        }
        let enc = w.finish(None, |_| true);
        v.sort_by_key(|(t, _)| *t);
        let got: std::vec::Vec<(u32, std::vec::Vec<u8>)> = record::parse(&enc)
            .unwrap()
            .into_iter()
            .map(|(t, wire, b)| {
                assert_eq!(wire, Wire::Scalar);
                (t, b.to_vec())
            })
            .collect();
        assert_eq!(got, v);
    });
}

/// How schema S1 wrote a tag (kind-marker harness).
#[derive(Clone, Copy, Debug, PartialEq, Eq, TypeGenerator)]
pub enum Wrote {
    Absent,
    /// A one-byte scalar (`u8`). Before the wire type, values `0x81..=0x87` looked like a kind
    /// marker (the round-5 false positive).
    Scalar(u8),
    /// A three-byte scalar spelling a marker field, `0D 01 kind` (tag 3, wire 1): before the
    /// wire type, the nested-record search could read such bytes as a record holding a marker.
    ScalarLikeRecord(u8),
    /// A collection field of kind `0x81 + (k % 7)`.
    Collection(u8),
    /// A nested record (wire 2) holding, at its tag 3, a collection marker (`true`) or a
    /// one-byte scalar (`false`) of value `kind_of(k)`.
    Nested(bool, u8),
}

/// How schema S2 declares a tag (kind-marker harness).
#[derive(Clone, Copy, Debug, PartialEq, Eq, TypeGenerator)]
pub enum Declares {
    Unknown,
    Scalar,
    Collection(u8),
    Reserved,
}

fn kind_of(k: u8) -> u8 {
    record::marker::MAP + k % 7
}

/// The nested record `Wrote::Nested(marker, k)` writes, through a record writer.
fn nested_bytes(marker: bool, k: u8) -> std::vec::Vec<u8> {
    let mut w = RecordWriter::new();
    if marker { w.push_marker(3, kind_of(k)) } else { w.push(3, vec![kind_of(k)]) }
    w.finish(None, |_| true)
}

/// The removal check the derive expands to (`__unknown_collection`), over the kernel helpers:
/// the first tag S2 neither declares nor reserves whose value is a wire-1 kind marker (or a
/// wire-2 nested record holding one).
fn first_unknown_collection(bytes: &[u8], s2: &[Declares; TAGS]) -> Option<(std::vec::Vec<u32>, u8)> {
    let f = p::parse(bytes).ok()?;
    for &(t, w, b) in f.iter() {
        let declared = (t as usize) <= TAGS && s2[t as usize - 1] != Declares::Unknown;
        if !declared && let Some((mut trail, k)) = p::unknown_field_collection(w, b) {
            trail.insert(0, t);
            return Some((trail, k));
        }
    }
    None
}

/// API round 5 (A3), the record-format side of "removal fails closed on unknown collection
/// tags", with the Verification round 2 wire type. Random engine, 20,000 cases: S1 writes up to
/// 4 tags, each absent, a one-byte scalar, a three-byte scalar spelling a marker field, a
/// collection field, or a nested record holding a marker or a scalar; S2 declares each tag
/// unknown, scalar, collection or reserved.
/// - A record writer emits exactly `varint(tag << 2 | 1) 01 kind` for each collection field (3
///   bytes for tags up to 31) and a root writer emits none; the result is canonical (parses,
///   re-encodes to itself) and each field parses back with the wire type it was written with.
/// - The removal check under S2 fires iff some tag S2 leaves unknown holds a marker: an S1
///   collection field, or a nested record holding one. A scalar NEVER fires, whatever its
///   bytes (the round-5 false positives, `u8` 0x81..=0x87 and record-shaped scalar bytes, are
///   gone). Reserved and declared tags never fire.
/// - S2's read-modify-write keeps unknown fields byte-for-byte with their wire type, drops
///   reserved tags, and writes its own marker for each collection it declares (the stored one
///   is not merged).
#[test]
fn g6_kind_markers_are_wire_typed_preserved_and_checked_on_removal() {
    let g = bolero::produce::<([Wrote; TAGS], [Declares; TAGS])>();
    bolero::check!().with_generator(g).with_iterations(20_000).for_each(|(s1, s2)| {
        // S1 writes, as `encode_field` does
        let write = |root: bool| {
            let mut w = if root { RecordWriter::root() } else { RecordWriter::new() };
            for (i, x) in s1.iter().enumerate() {
                let t = i as u32 + 1;
                match x {
                    Wrote::Absent => {}
                    Wrote::Scalar(v) => w.push(t, vec![*v]),
                    Wrote::ScalarLikeRecord(k) => w.push(t, vec![0x0D, 0x01, kind_of(*k)]),
                    Wrote::Collection(k) => w.push_marker(t, kind_of(*k)),
                    Wrote::Nested(m, k) => w.push_record(t, nested_bytes(*m, *k)),
                }
            }
            w.finish(None, |_| true)
        };
        let stored = write(false);
        let as_root = write(true);
        let f = record::parse(&stored).expect("own encoding parses");
        let size = |x: &Wrote, root: bool| match x {
            Wrote::Absent => 0,
            Wrote::Scalar(_) => 3,
            Wrote::Collection(_) => if root { 0 } else { 3 },
            Wrote::ScalarLikeRecord(_) | Wrote::Nested(..) => 5,
        };
        assert_eq!(stored.len(), s1.iter().map(|x| size(x, false)).sum::<usize>(), "one-byte field keys and lengths");
        assert_eq!(as_root.len(), s1.iter().map(|x| size(x, true)).sum::<usize>(), "a root leaves no markers");
        for (i, x) in s1.iter().enumerate() {
            let t = i as u32 + 1;
            let got = f.iter().find(|(ft, _, _)| *ft == t).map(|(_, w, b)| (*w, b.to_vec()));
            let want = match x {
                Wrote::Absent => None,
                Wrote::Scalar(v) => Some((Wire::Scalar, vec![*v])),
                Wrote::ScalarLikeRecord(k) => Some((Wire::Scalar, vec![0x0D, 0x01, kind_of(*k)])),
                Wrote::Collection(k) => Some((Wire::Marker, vec![kind_of(*k)])),
                Wrote::Nested(m, k) => Some((Wire::Record, nested_bytes(*m, *k))),
            };
            assert_eq!(got, want, "tag {t}");
        }
        let mut w = RecordWriter::new();
        for (t, wire, b) in &f {
            w.push_wire(*t, *wire, b.to_vec());
        }
        assert_eq!(w.finish(None, |_| true), stored, "canonical");

        // the removal check under S2: only real markers, never a scalar
        let want = (0..TAGS).find_map(|i| match (s1[i], s2[i]) {
            (Wrote::Collection(k), Declares::Unknown) => Some((vec![i as u32 + 1], kind_of(k))),
            (Wrote::Nested(true, k), Declares::Unknown) => Some((vec![i as u32 + 1, 3], kind_of(k))),
            _ => None,
        });
        assert_eq!(first_unknown_collection(&stored, s2), want);

        // S2 rewrites (read-modify-write, merging the stored bytes)
        let mut w = RecordWriter::new();
        for (i, d) in s2.iter().enumerate() {
            let t = i as u32 + 1;
            match d {
                Declares::Scalar => w.push(t, vec![0x07]),
                Declares::Collection(k) => w.push_marker(t, kind_of(*k)),
                Declares::Unknown | Declares::Reserved => {}
            }
        }
        let keep = |t: u32| (t as usize) <= TAGS && s2[t as usize - 1] == Declares::Unknown;
        let rewritten = w.finish(Some(&stored), keep);
        let after = record::parse(&rewritten).expect("parses");
        for i in 0..TAGS {
            let t = i as u32 + 1;
            let got = after.iter().find(|(x, _, _)| *x == t).copied();
            match s2[i] {
                Declares::Unknown => {
                    assert_eq!(got, f.iter().find(|(x, _, _)| *x == t).copied(), "unknown tag {t} not preserved")
                }
                Declares::Reserved => assert_eq!(got, None, "reserved tag {t} not dropped"),
                Declares::Scalar => assert_eq!(got, Some((t, Wire::Scalar, &[0x07][..]))),
                Declares::Collection(k) => {
                    assert_eq!(got, Some((t, Wire::Marker, &[kind_of(k)][..])), "S2's own marker")
                }
            }
        }
    });
}
