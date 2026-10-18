//! Guarantee 6, Permanent layout (the record-codec kernel). A deploy never moves or rewrites
//! existing data by itself: decoding is a pure function of the stored bytes (no host call can
//! happen in it), missing tags default (or fail naming the tag if `required`), unknown tags are
//! preserved through a read-modify-write, reserved tags are dropped, and every record has
//! exactly one encoding. Drives the production codec (`record::RecordWriter`, `record::parse`)
//! and the helpers the derive expands to (`root::__private::{field, field_or,
//! field_required}`). The derive's own output is checked in `tests/state_v2_verify.rs`.

use crate::state::key::{DEFAULT_ROOT, Path};
use crate::state::record::{self, RecordError, RecordWriter};
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
}

const NF: usize = if cfg!(kani) { 2 } else { 4 };

fn distinct(fields: &[Field]) -> Option<std::vec::Vec<(u32, std::vec::Vec<u8>)>> {
    let mut v: std::vec::Vec<(u32, std::vec::Vec<u8>)> = std::vec::Vec::new();
    for f in fields.iter().filter(|f| f.present) {
        if v.iter().any(|(t, _)| *t == f.tag) {
            return None;
        }
        v.push((f.tag, f.body[..f.len as usize].to_vec()));
    }
    Some(v)
}

/// Encode then parse is the identity (fields come back sorted by tag), whatever order they
/// were pushed in. Bound (Kani): up to 2 fields, tags in 1..300 (one- and two-byte LEB128),
/// bodies of 0..=4 bytes (4 fields x 16 bytes did not finish symbolic execution in 15 min).
/// Random engine: up to 4 fields, bodies 0..=16, 20,000 records.
#[test]
#[cfg_attr(kani, kani::proof)]
#[cfg_attr(kani, kani::unwind(8))]
fn g6_record_encode_then_parse_is_identity() {
    bolero::check!().with_type::<[Field; NF]>().with_iterations(20_000).for_each(|fields| {
        if cfg!(kani) && fields.iter().any(|f| f.len > 4) {
            return;
        }
        let Some(v) = distinct(fields) else { return };
        let mut w = RecordWriter::new();
        for (t, b) in &v {
            w.push(*t, b.clone());
        }
        let enc = w.finish(None, |_| true);
        let mut want = v.clone();
        want.sort_by_key(|(t, _)| *t);
        let got = record::parse(&enc).expect("own encoding parses");
        assert_eq!(got.len(), want.len());
        for ((gt, gb), (wt, wb)) in got.iter().zip(&want) {
            assert_eq!((*gt, *gb), (*wt, &wb[..]));
        }
    });
}

/// Canonical form: any byte string that parses re-encodes to exactly itself (after the
/// optional leading clearing flag), so a record has one encoding. Bound (Kani): every byte
/// string of length 0..=6. Random engine: 50,000 strings of up to 64 bytes.
#[test]
#[cfg_attr(kani, kani::proof)]
#[cfg_attr(kani, kani::unwind(9))]
fn g6_record_parse_accepts_only_canonical_encodings() {
    let max = if cfg!(kani) { 6 } else { 64 };
    bolero::check!().with_generator(bolero::produce::<std::vec::Vec<u8>>().with().len(0usize..=max)).with_iterations(50_000).for_each(|b| {
        if let Ok(fields) = record::parse(b) {
            let mut w = RecordWriter::new();
            for (t, body) in &fields {
                w.push(*t, body.to_vec());
            }
            let enc = w.finish(None, |_| true);
            let start = if record::is_clearing(b) { 2 } else { 0 };
            assert_eq!(&enc[..], &b[start..], "two encodings of one record");
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

/// Tags in the schema-evolution harness: 4 (random engine), 3 (Kani; 4 did not finish
/// symbolic execution in 15 min).
const TAGS: usize = if cfg!(kani) { 3 } else { 4 };

/// Schema evolution, the kernel the derive expands to. A record written by schema S1 (any
/// subset of tags 1..=4, `u8` fields) is read by schema S2 (each tag Unknown, Default,
/// DefaultExpr, Required or Reserved): present tags decode to their value, missing ones to
/// their default, a missing required tag fails with `Missing { tag }` naming it. S2 then
/// rewrites the record (every known value + 1) merging the original bytes: unknown tags keep
/// their exact bytes, reserved tags are dropped, known tags carry the new values. Reading the
/// rewrite back with S1 (a rollback) sees its own unknown-to-S2 fields intact. Bound (Kani):
/// all 2^3 written subsets x 5^3 schemas x u8 values (tags 1..=3). Random engine: 20,000 cases.
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

/// Random engine only (20,000 cases): any `u32` tag and bodies up to 300 bytes, so the length
/// varint crosses 127/128; encode then parse is the identity.
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
        let got: std::vec::Vec<(u32, std::vec::Vec<u8>)> = record::parse(&enc).unwrap().into_iter().map(|(t, b)| (t, b.to_vec())).collect();
        assert_eq!(got, v);
    });
}
