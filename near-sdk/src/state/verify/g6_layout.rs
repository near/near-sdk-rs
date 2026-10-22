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

/// The `(tag, wire, body)` a generated field stands for (a marker's body is cut to 1 byte).
fn entry_of(f: &Field) -> (u32, Wire, std::vec::Vec<u8>) {
    let wire = wire_of(f.wire);
    let len = if wire == Wire::Marker { 1 } else { f.len as usize };
    (f.tag, wire, f.body[..len].to_vec())
}

fn distinct(fields: &[Field]) -> Option<std::vec::Vec<(u32, Wire, std::vec::Vec<u8>)>> {
    let mut v: std::vec::Vec<(u32, Wire, std::vec::Vec<u8>)> = std::vec::Vec::new();
    for f in fields.iter().filter(|f| f.present) {
        if v.iter().any(|(t, _, _)| *t == f.tag) {
            return None;
        }
        v.push(entry_of(f));
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
        identity_check(&v);
    });
}

/// The encode-then-parse property, shared by the random-engine harness and the shrunk Kani
/// proof: `v` (distinct tags, any order) encodes and parses back sorted by tag, wire types and
/// bodies intact.
fn identity_check(v: &[(u32, Wire, std::vec::Vec<u8>)]) {
    let mut w = RecordWriter::new();
    for (t, wire, b) in v {
        w.push_wire(*t, *wire, b.clone());
    }
    let enc = w.finish(None, |_| true);
    let got = record::parse(&enc).expect("own encoding parses");
    // `parse` only accepts strictly increasing tags, so "same length, and every parsed field is
    // one of `v` with its wire type and bytes" is "the fields of `v`, sorted by tag" (the tags of
    // `v` are distinct), without cloning and sorting `v`.
    assert_eq!(got.len(), v.len());
    for (i, (gt, gw, gb)) in got.iter().enumerate() {
        assert!(i == 0 || got[i - 1].0 < *gt, "tags ascending");
        let (_, ww, wb) = v.iter().find(|(t, _, _)| t == gt).expect("a parsed tag was pushed");
        assert!(*gw == *ww && *gb == &wb[..], "tag {gt}: wire type and bytes");
    }
}

/// Every field shape at the shrunk bound, `(wire, body length)`: a scalar or a nested record
/// with a body of 0 or 1 byte, or a one-byte marker (5 shapes). Concrete, so every buffer the
/// codec allocates has a constant size.
#[cfg(kani)]
const FIELD_SHAPES: [(u8, u8); 5] = [(0, 0), (0, 1), (2, 0), (2, 1), (1, 1)];

/// One field of the given shape with a symbolic tag in 1..=127 and symbolic body bytes.
#[cfg(kani)]
fn shaped_field((wire, len): (u8, u8)) -> Field {
    let tag: u8 = kani::any();
    kani::assume((1..=127).contains(&tag));
    let b: [u8; 4] = kani::any();
    let mut body = [0u8; 16];
    body[..4].copy_from_slice(&b);
    Field { tag: tag as u32, body, len, present: true, wire }
}

/// Encode then parse is the identity, Kani at a shrunk bound, as 5 proofs `..._small_s00` to
/// `..._small_s04` (one per field shape; `--harness g6_record_encode_then_parse_is_identity_small`
/// runs them all). Together: the empty record and every one-field record, the field of any of
/// the 5 [`FIELD_SHAPES`] (any of the 3 wire types, a body of 0 or 1 symbolic byte, a marker 1)
/// with a symbolic tag in 1..=127 (field keys of one or two bytes, since the key is
/// `tag << 2 | wire`). Status (Oct 20, Kani 0.68, CBMC 6.11, 64 GB host): all 5 VERIFIED, 105
/// to 169 s each, 629 s in total, peak 34 GB. Larger bounds did not finish: one field with a
/// 4-byte body reached 48-50 GB with no verdict in 15 minutes, and a pair of concrete 4-byte
/// fields passed 50 GB in propositional reduction.
macro_rules! g6_identity_small_proofs {
    ($($name:ident = $i:literal),* $(,)?) => {
        $(
            #[cfg(kani)]
            #[kani::proof]
            #[kani::unwind(3)]
            fn $name() {
                if $i == 0 {
                    identity_check(&[]);
                }
                identity_check(&[entry_of(&shaped_field(FIELD_SHAPES[$i]))]);
            }
        )*
    };
}

g6_identity_small_proofs!(
    g6_record_encode_then_parse_is_identity_small_s00 = 0,
    g6_record_encode_then_parse_is_identity_small_s01 = 1,
    g6_record_encode_then_parse_is_identity_small_s02 = 2,
    g6_record_encode_then_parse_is_identity_small_s03 = 3,
    g6_record_encode_then_parse_is_identity_small_s04 = 4
);

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
        canonical_check(&arr[..*n as usize % (CAN_MAX + 1)]);
    });
}

/// The canonical-form property, shared by the random-engine harness and the shrunk Kani
/// proofs: if `b` parses, it re-encodes to exactly itself.
fn canonical_check(b: &[u8]) {
    if let Ok(fields) = record::parse(b) {
        reencodes_to(b, fields);
    }
}

/// `fields`, parsed from `b`, re-encode to exactly `b`.
fn reencodes_to<'a>(b: &[u8], fields: impl IntoIterator<Item = record::Field<'a>>) {
    let mut w = RecordWriter::new();
    for (t, wire, body) in fields {
        assert!(wire != Wire::Marker || body.len() == 1, "a marker is one byte");
        w.push_wire(t, wire, body.to_vec());
    }
    let enc = w.finish(None, |_| true);
    assert_eq!(&enc[..], b, "two encodings of one record");
}

/// Canonical form over every byte string of length exactly `N <= 3` (shrunk Kani proofs): the
/// same property as [`canonical_check`], with the parsed field count and body length made
/// concrete before the re-encode (a field takes at least 2 bytes, so 3 bytes hold at most one
/// field with at most a one-byte body, which the proof also checks), so the production writer
/// copies buffers of known size.
#[cfg(kani)]
fn small_canonical<const N: usize>() {
    let arr: [u8; N] = kani::any();
    if let Ok(fields) = record::parse(&arr) {
        match fields.len() {
            // arrays, not slices of the parsed vector: their iteration count is a constant; and
            // the one field's body length (0 or 1 in 3 bytes) is made concrete the same way
            0 => reencodes_to(&arr, []),
            1 => {
                let (t, w, body) = fields[0];
                match body.len() {
                    0 => reencodes_to(&arr, [(t, w, &body[..0])]),
                    1 => reencodes_to(&arr, [(t, w, &body[..1])]),
                    _ => panic!("{N} bytes hold a body of at most one byte"),
                }
            }
            _ => panic!("{N} bytes hold more than one field"),
        }
    }
}

/// Canonical form, Kani at a shrunk bound, as 4 proofs `..._small_len0` to `..._small_len3`:
/// together, every byte string of length 0..=3 that parses re-encodes to exactly itself.
/// Status (Oct 20, Kani 0.68, CBMC 6.11, 64 GB host): `len0` VERIFIED (0.6 s); `len1` no
/// verdict, still in symbolic execution at the 60-minute cap (14 GB); `len2` no verdict at a
/// 15-minute cap; `len3` no verdict in 4 attempts of 10 minutes.
macro_rules! g6_canonical_small_proofs {
    ($($name:ident = $n:literal),* $(,)?) => {
        $(
            #[cfg(kani)]
            #[kani::proof]
            #[kani::unwind(4)]
            fn $name() {
                small_canonical::<$n>();
            }
        )*
    };
}

g6_canonical_small_proofs!(
    g6_record_parse_accepts_only_canonical_encodings_small_len0 = 0,
    g6_record_parse_accepts_only_canonical_encodings_small_len1 = 1,
    g6_record_parse_accepts_only_canonical_encodings_small_len2 = 2,
    g6_record_parse_accepts_only_canonical_encodings_small_len3 = 3,
);

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
    bolero::check!().with_generator(g).with_iterations(20_000).for_each(|(s1, s2)| evolution_check(s1, s2));
}

/// The schema-evolution property for `T` tags, shared by the random-engine harness and the
/// shrunk Kani proofs.
fn evolution_check<const T: usize>(s1: &[Option<u8>; T], s2: &[Decl; T]) {
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
    let mut decoded = [None::<u8>; T];
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
    let rewritten = w.finish(Some(&stored), |t| t as usize <= T && !known(t) && s2[t as usize - 1] != Decl::Reserved);
    let after = record::parse(&rewritten).expect("parses");
    for i in 0..T {
        let tag = i as u32 + 1;
        let got = record::find(&after, tag);
        match s2[i] {
            Decl::Unknown => assert_eq!(got, s1[i].as_ref().map(core::slice::from_ref), "unknown tag {tag} not preserved"),
            Decl::Reserved => assert_eq!(got, None, "reserved tag {tag} not dropped"),
            _ => assert_eq!(got, Some(&[decoded[i].unwrap().wrapping_add(1)][..])),
        }
    }
}

/// Schema evolution, Kani at a shrunk bound, as 10 proofs `..._small_{absent,written}_<decl>`
/// (`--harness g6_schema_evolution_defaults_required_unknown_reserved_small` runs them all).
/// Together: ONE tag (tag 1), absent from the S1 record or written with any `u8` value, and
/// declared by S2 as each of the 5 `Decl`s. One case per proof, so every record the codec
/// builds has a known shape and only the value is symbolic (all 10 cases in one proof were
/// still in symbolic execution after 20 minutes). Status (Oct 20, Kani 0.68, CBMC 6.11, 64 GB
/// host): NO VERDICT; `..._small_written_unknown` was still in symbolic execution at the
/// 60-minute cap (15 GB), and the other nine were not run.
macro_rules! g6_evolution_small_proofs {
    ($($name:ident = $written:literal, $d:ident),* $(,)?) => {
        $(
            #[cfg(kani)]
            #[kani::proof]
            #[kani::unwind(3)]
            fn $name() {
                let s1 = if $written { Some(kani::any::<u8>()) } else { None };
                evolution_check(&[s1], &[Decl::$d]);
            }
        )*
    };
}

g6_evolution_small_proofs!(
    g6_schema_evolution_defaults_required_unknown_reserved_small_absent_unknown = false, Unknown,
    g6_schema_evolution_defaults_required_unknown_reserved_small_written_unknown = true, Unknown,
    g6_schema_evolution_defaults_required_unknown_reserved_small_absent_default = false, Default,
    g6_schema_evolution_defaults_required_unknown_reserved_small_written_default = true, Default,
    g6_schema_evolution_defaults_required_unknown_reserved_small_absent_default_expr = false, DefaultExpr,
    g6_schema_evolution_defaults_required_unknown_reserved_small_written_default_expr = true, DefaultExpr,
    g6_schema_evolution_defaults_required_unknown_reserved_small_absent_required = false, Required,
    g6_schema_evolution_defaults_required_unknown_reserved_small_written_required = true, Required,
    g6_schema_evolution_defaults_required_unknown_reserved_small_absent_reserved = false, Reserved,
    g6_schema_evolution_defaults_required_unknown_reserved_small_written_reserved = true, Reserved
);

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
        for (i, d) in s2.iter().enumerate() {
            let t = i as u32 + 1;
            let got = after.iter().find(|(x, _, _)| *x == t).copied();
            match *d {
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
