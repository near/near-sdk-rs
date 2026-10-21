//! Tagged record encoding.
//!
//! ```text
//! record  := field*
//! field   := varint(tag << 2 | wire) varint(len) bytes[len]
//! wire    := 0  scalar: a leaf's Borsh bytes (or a `legacy` field's)
//!          | 1  kind marker: len is 1, the byte names a collection kind (`marker`)
//!          | 2  nested record: the bytes are a record (`#[near(storage)]` struct inline)
//!          | 3  reserved, rejected
//! ```
//!
//! Canonical form, so that every record has exactly one encoding:
//! - varints are minimal LEB128 (no trailing 0x00 continuation group); a tag is a `u32`, so the
//!   field key `tag << 2 | wire` is below 2^34, and a length fits in a `u32`;
//! - tag 0 is rejected, wire 3 is rejected, and a marker has length exactly 1;
//! - tags are strictly ascending (the spec does not say this; without it, field order would
//!   be a second encoding of the same record, see PROTOTYPE_NOTES.md). A tag appears once,
//!   whatever its wire type.
//!
//! The D9 clearing flag is not in the record: a record being cleared across calls is stored at
//! its flag key (`Path::flag`, the record key plus `00`), so its bytes are the plain record.
//!
//! Kind markers (API round 5, hazard A3; wire type since Verification round 2): a collection or
//! `Lazy` field of a `#[near(storage)]` record stores nothing inline, but leaves a wire-1 field
//! `varint(tag << 2 | 1) 01 kind` in the record, one byte naming its collection kind
//! ([`marker`]). Removal reads the stored record and refuses one that carries an unknown tag of
//! wire type 1 (at any depth through wire-2 nested records): it is a collection some other
//! version of the contract declared, and removing the record would orphan its keys. The wire
//! type is what makes this unambiguous: a scalar (wire 0) is never read as a marker or searched
//! as a nested record, whatever its bytes. The marker is not part of any key, and the shifted
//! field key is record-only (keys keep `LEB128(tag)`). A root record carries no markers (a root
//! is never removed).

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecordError {
    Truncated,
    NonCanonicalVarint,
    VarintOverflow,
    TagZero,
    /// Tags must be strictly ascending.
    TagOrder { prev: u32, tag: u32 },
    /// Wire type 3 is reserved.
    ReservedWire { tag: u32 },
    /// A kind marker (wire 1) must be exactly one byte.
    MarkerLength { tag: u32, len: u32 },
    /// Borsh decode of a scalar field failed.
    Field { tag: u32, msg: String },
    /// A required field (no `Default`) is missing.
    Missing { tag: u32 },
    /// Trailing bytes after a leaf value.
    Leaf(String),
}

impl core::fmt::Display for RecordError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::fmt::Debug::fmt(self, f)?;
        if let RecordError::Missing { tag } = self {
            write!(
                f,
                ": required field tag {tag} is absent from the stored record; `required` is only \
                 for fields present since the record's first write, a field added later needs \
                 `#[key({tag}, default = ..)]`, a `Default` type, or `Option`"
            )?;
        }
        Ok(())
    }
}

/// The wire type of a record field, the low two bits of its field key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Wire {
    /// A leaf value's bytes. Never inspected by the removal check.
    Scalar = 0,
    /// A collection kind marker: exactly one byte, a [`marker`] value (or one a later
    /// version defines).
    Marker = 1,
    /// A nested record (`#[near(storage)]` struct stored inline): the bytes are a record.
    Record = 2,
}

impl Wire {
    const fn from_bits(b: u64) -> Option<Wire> {
        match b {
            0 => Some(Wire::Scalar),
            1 => Some(Wire::Marker),
            2 => Some(Wire::Record),
            _ => None,
        }
    }
}

/// The field key of `tag` with wire type `wire`: `tag << 2 | wire`.
pub const fn field_key(tag: u32, wire: Wire) -> u64 {
    (tag as u64) << 2 | wire as u64
}

pub fn write_varint(out: &mut Vec<u8>, mut v: u64) {
    loop {
        let byte = (v & 0x7F) as u8;
        v >>= 7;
        if v == 0 {
            out.push(byte);
            return;
        }
        out.push(byte | 0x80);
    }
}

pub fn varint_len(v: u64) -> usize {
    let mut n = 1;
    let mut v = v >> 7;
    while v != 0 {
        n += 1;
        v >>= 7;
    }
    n
}

/// Reads a canonical LEB128 varint of at most 5 bytes (below 2^35).
fn read_varint_35(bytes: &[u8], pos: &mut usize) -> Result<u64, RecordError> {
    let mut result: u64 = 0;
    let mut shift = 0u32;
    let start = *pos;
    loop {
        let byte = *bytes.get(*pos).ok_or(RecordError::Truncated)?;
        *pos += 1;
        if shift >= 35 {
            return Err(RecordError::VarintOverflow);
        }
        result |= ((byte & 0x7F) as u64) << shift;
        shift += 7;
        if byte & 0x80 == 0 {
            // A multi-byte varint whose final group is zero has a shorter encoding.
            if *pos - start > 1 && byte == 0 {
                return Err(RecordError::NonCanonicalVarint);
            }
            return Ok(result);
        }
    }
}

/// Reads a canonical LEB128 varint that fits in u32.
pub fn read_varint_u32(bytes: &[u8], pos: &mut usize) -> Result<u32, RecordError> {
    u32::try_from(read_varint_35(bytes, pos)?).map_err(|_| RecordError::VarintOverflow)
}

/// One parsed field: its tag, wire type and bytes (for a marker, the one kind byte).
pub type Field<'a> = (u32, Wire, &'a [u8]);

/// Parses a record into `(tag, wire, bytes)` fields, enforcing the canonical form.
pub fn parse(bytes: &[u8]) -> Result<Vec<Field<'_>>, RecordError> {
    let mut out = Vec::new();
    let mut pos = 0;
    let mut prev = 0u32;
    while pos < bytes.len() {
        let key = read_varint_35(bytes, &mut pos)?;
        let tag = u32::try_from(key >> 2).map_err(|_| RecordError::VarintOverflow)?;
        if tag == 0 {
            return Err(RecordError::TagZero);
        }
        if tag <= prev {
            return Err(RecordError::TagOrder { prev, tag });
        }
        let wire = Wire::from_bits(key & 3).ok_or(RecordError::ReservedWire { tag })?;
        prev = tag;
        let len = read_varint_u32(bytes, &mut pos)?;
        if wire == Wire::Marker && len != 1 {
            return Err(RecordError::MarkerLength { tag, len });
        }
        let end = pos.checked_add(len as usize).ok_or(RecordError::Truncated)?;
        let body = bytes.get(pos..end).ok_or(RecordError::Truncated)?;
        out.push((tag, wire, body));
        pos = end;
    }
    Ok(out)
}

/// The one-byte kind markers a collection or `Lazy` field leaves under its tag (wire type 1)
/// in its parent record (API round 5). Since the wire type says a field is a marker, the byte
/// only names the kind: no scalar can be mistaken for one. The values are part of the frozen
/// record format. A marker byte this code does not know (a kind a later version adds) is still
/// a marker, and removal still refuses it.
pub mod marker {
    pub const MAP: u8 = 0x81;
    pub const LOOKUP_MAP: u8 = 0x82;
    pub const SET: u8 = 0x83;
    pub const LOOKUP_SET: u8 = 0x84;
    pub const VECTOR: u8 = 0x85;
    pub const LAZY: u8 = 0x86;
    pub const GROUPED_MAP: u8 = 0x87;

    /// Whether `b` is a kind this version defines.
    pub const fn is_marker(b: u8) -> bool {
        b >= MAP && b <= GROUPED_MAP
    }

    /// The collection type a marker names, for error messages.
    pub const fn name(b: u8) -> &'static str {
        match b {
            MAP => "Map",
            LOOKUP_MAP => "LookupMap",
            SET => "Set",
            LOOKUP_SET => "LookupSet",
            VECTOR => "Vector",
            LAZY => "Lazy",
            GROUPED_MAP => "GroupedMap",
            _ => "unknown collection",
        }
    }
}

/// Whether the value of a tag the code does not know is (or holds) a collection: a wire-1
/// kind marker, or a wire-2 nested record holding one at any depth. Returns the tags below this
/// one leading to the marker, and the marker. A wire-0 scalar never matches, whatever its
/// bytes, and neither does a nested record that does not parse.
pub fn unknown_value_collection(wire: Wire, body: &[u8]) -> Option<(Vec<u32>, u8)> {
    match wire {
        Wire::Scalar => None,
        Wire::Marker => body.first().map(|k| (Vec::new(), *k)),
        Wire::Record => {
            for (t, w, inner) in parse(body).ok()? {
                if let Some((mut trail, k)) = unknown_value_collection(w, inner) {
                    trail.insert(0, t);
                    return Some((trail, k));
                }
            }
            None
        }
    }
}

/// Builds a record from known fields, then merges in unknown fields from the bytes
/// previously stored at the same key.
pub struct RecordWriter {
    fields: Vec<(u32, Wire, Vec<u8>)>,
    /// Whether collection fields leave kind markers: yes for `#[near(storage)]` records, no for
    /// a root (never removed, so its markers would buy nothing).
    markers: bool,
}

impl Default for RecordWriter {
    fn default() -> Self {
        Self::new()
    }
}

impl RecordWriter {
    /// A writer for a `#[near(storage)]` record: collection fields leave kind markers.
    pub fn new() -> Self {
        RecordWriter { fields: Vec::new(), markers: true }
    }

    /// A writer for a contract root: no kind markers.
    pub fn root() -> Self {
        RecordWriter { fields: Vec::new(), markers: false }
    }

    /// A scalar field (wire 0).
    pub fn push(&mut self, tag: u32, bytes: Vec<u8>) {
        self.fields.push((tag, Wire::Scalar, bytes));
    }

    /// A nested record field (wire 2): `bytes` is the nested record's own encoding.
    pub fn push_record(&mut self, tag: u32, bytes: Vec<u8>) {
        self.fields.push((tag, Wire::Record, bytes));
    }

    /// Records that the collection field `tag` is declared: `tag << 2 | 1, 01, kind`, unless
    /// this writer is a root's.
    pub fn push_marker(&mut self, tag: u32, kind: u8) {
        debug_assert!(marker::is_marker(kind), "collection kind marker {kind:#x}");
        if self.markers {
            self.fields.push((tag, Wire::Marker, vec![kind]));
        }
    }

    /// A field with an explicit wire type, as parsed (re-encoding a parsed record).
    pub fn push_wire(&mut self, tag: u32, wire: Wire, bytes: Vec<u8>) {
        assert!(wire != Wire::Marker || bytes.len() == 1, "a kind marker is one byte");
        self.fields.push((tag, wire, bytes));
    }

    /// `keep_unknown(tag)` decides whether a field found in `orig` but not pushed is preserved
    /// (with its wire type). The derive passes "not a known inline tag and not reserved".
    pub fn finish(mut self, orig: Option<&[u8]>, keep_unknown: impl Fn(u32) -> bool) -> Vec<u8> {
        if let Some(orig) = orig
            && let Ok(old) = parse(orig)
        {
            for (tag, wire, body) in old {
                if keep_unknown(tag) && !self.fields.iter().any(|(t, _, _)| *t == tag) {
                    self.fields.push((tag, wire, body.to_vec()));
                }
            }
        }
        self.fields.sort_by_key(|(t, _, _)| *t);
        for w in self.fields.windows(2) {
            assert!(w[0].0 != w[1].0, "duplicate record tag {}", w[0].0);
        }
        let mut out = Vec::new();
        for (tag, wire, body) in &self.fields {
            write_varint(&mut out, field_key(*tag, *wire));
            write_varint(&mut out, body.len() as u64);
            out.extend_from_slice(body);
        }
        out
    }
}

/// The A3 first-write rule (round 7): the stored record `bytes` with the kind marker `kind`
/// at the field reached by `trail` (tags from this record down through wire-2 nested records to
/// the collection field), or `None` when that marker is already there. A missing nested record
/// on the way is created holding only what leads to the marker; a field on the way stored under
/// another wire type (a re-used tag, which `storage diff` refuses) is replaced, as a rewrite of
/// the record through `get_mut` would replace it. Every other field is kept byte for byte.
pub fn with_marker(bytes: &[u8], trail: &[u32], kind: u8) -> Result<Option<Vec<u8>>, RecordError> {
    let (&tag, rest) = trail.split_first().expect("a marker trail names at least the field");
    let fields = parse(bytes)?;
    let body = match (fields.iter().find(|(t, _, _)| *t == tag), rest.is_empty()) {
        (Some((_, Wire::Marker, _)), true) => return Ok(None),
        (_, true) => (Wire::Marker, vec![kind]),
        (Some((_, Wire::Record, inner)), false) => match with_marker(inner, rest, kind)? {
            None => return Ok(None),
            Some(b) => (Wire::Record, b),
        },
        (_, false) => (Wire::Record, with_marker(&[], rest, kind)?.expect("an empty record has no marker")),
    };
    let mut w = RecordWriter::new();
    for (t, wire, b) in fields {
        if t != tag {
            w.push_wire(t, wire, b.to_vec());
        }
    }
    w.push_wire(tag, body.0, body.1);
    Ok(Some(w.finish(None, |_| false)))
}

/// Finds the bytes of one field in a parsed record (any wire type).
pub fn find<'a>(fields: &[Field<'a>], tag: u32) -> Option<&'a [u8]> {
    fields.iter().find(|(t, _, _)| *t == tag).map(|(_, _, b)| *b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn varint_round_trip_and_canonical() {
        for v in [1u32, 127, 128, 300, 16_383, 16_384, u32::MAX] {
            let mut b = Vec::new();
            write_varint(&mut b, v as u64);
            assert_eq!(b.len(), varint_len(v as u64));
            let mut pos = 0;
            assert_eq!(read_varint_u32(&b, &mut pos).unwrap(), v);
            assert_eq!(pos, b.len());
        }
        // 1 encoded with a redundant continuation group.
        assert_eq!(read_varint_u32(&[0x81, 0x00], &mut 0), Err(RecordError::NonCanonicalVarint));
        // 0 encoded in two bytes.
        assert_eq!(read_varint_u32(&[0x80, 0x00], &mut 0), Err(RecordError::NonCanonicalVarint));
        assert_eq!(read_varint_u32(&[0x80], &mut 0), Err(RecordError::Truncated));
        assert_eq!(
            read_varint_u32(&[0xFF, 0xFF, 0xFF, 0xFF, 0x10], &mut 0),
            Err(RecordError::VarintOverflow)
        );
        // a sixth byte is an overflow, not a longer varint
        assert_eq!(
            read_varint_u32(&[0x80, 0x80, 0x80, 0x80, 0x80, 0x01], &mut 0),
            Err(RecordError::VarintOverflow)
        );
    }

    /// D9 moved the clearing flag out of the record (to its own key), so tag 0 is rejected
    /// everywhere again, including the old leading `00 00`, under every wire type.
    #[test]
    fn tag_zero_is_rejected_even_leading_and_empty() {
        assert_eq!(parse(&[0x00, 0x00]), Err(RecordError::TagZero));
        assert_eq!(parse(&[0x01, 0x01, 0x81]), Err(RecordError::TagZero));
        assert_eq!(parse(&[0x02, 0x00]), Err(RecordError::TagZero));
        assert_eq!(parse(&[0x00, 0x00, 0x04, 0x01, 0x07]), Err(RecordError::TagZero));
        assert_eq!(parse(&[0x04, 0x00, 0x00, 0x00]), Err(RecordError::TagZero));
    }

    #[test]
    fn tag_zero_and_order_rejected() {
        assert_eq!(parse(&[0x00, 0x01, 0x05]), Err(RecordError::TagZero));
        assert_eq!(parse(&[0x08, 0x00, 0x04, 0x00]), Err(RecordError::TagOrder { prev: 2, tag: 1 }));
        assert_eq!(parse(&[0x04, 0x00, 0x04, 0x00]), Err(RecordError::TagOrder { prev: 1, tag: 1 }));
        // the same tag under two wire types is still a repeat
        assert_eq!(parse(&[0x04, 0x00, 0x05, 0x01, 0x81]), Err(RecordError::TagOrder { prev: 1, tag: 1 }));
        assert_eq!(parse(&[0x04, 0x05, 0x00]), Err(RecordError::Truncated));
    }

    #[test]
    fn wire_types_reserved_and_marker_length() {
        assert_eq!(parse(&[0x07, 0x00]), Err(RecordError::ReservedWire { tag: 1 }));
        assert_eq!(parse(&[0x05, 0x00]), Err(RecordError::MarkerLength { tag: 1, len: 0 }));
        assert_eq!(parse(&[0x05, 0x02, 0x81, 0x81]), Err(RecordError::MarkerLength { tag: 1, len: 2 }));
        assert_eq!(
            parse(&[0x04, 0x01, 0x81, 0x09, 0x01, 0x81, 0x0E, 0x00]).unwrap(),
            vec![(1, Wire::Scalar, &[0x81u8][..]), (2, Wire::Marker, &[0x81][..]), (3, Wire::Record, &[][..])]
        );
        // the largest tag: key (2^32 - 1) << 2 | 2 is a 5-byte varint
        let mut w = RecordWriter::new();
        w.push_record(u32::MAX, vec![]);
        let b = w.finish(None, |_| true);
        assert_eq!(b.len(), 6);
        assert_eq!(parse(&b).unwrap(), vec![(u32::MAX, Wire::Record, &[][..])]);
        // a key above u32::MAX << 2 | 3 overflows
        assert_eq!(parse(&[0x80, 0x80, 0x80, 0x80, 0x40, 0x00]), Err(RecordError::VarintOverflow));
    }

    /// "An Account with a nonce and a balance map is a 13-byte record": 10 for the nonce, 3 for
    /// the balance map's kind marker (API round 5; it was 10 before). With the wire type the
    /// bytes are `04 08 <8> 09 01 81`.
    #[test]
    fn account_record_is_thirteen_bytes() {
        let mut w = RecordWriter::new();
        w.push(1, borsh::to_vec(&7u64).unwrap());
        w.push_marker(2, marker::MAP);
        let b = w.finish(None, |_| true);
        assert_eq!(b.len(), 13);
        assert_eq!(&b[..2], &[0x04, 0x08]);
        assert_eq!(&b[10..], &[0x09, 0x01, marker::MAP]);
        // a root writes no marker
        let mut w = RecordWriter::root();
        w.push(1, borsh::to_vec(&7u64).unwrap());
        w.push_marker(2, marker::MAP);
        assert_eq!(w.finish(None, |_| true).len(), 10);
    }

    /// One-byte field keys hold tags 1..=31; tag 32 takes two bytes.
    #[test]
    fn one_byte_field_keys_up_to_tag_31() {
        for (tag, len) in [(1u32, 1usize), (31, 1), (32, 2), (4095, 2), (4096, 3)] {
            for wire in [Wire::Scalar, Wire::Marker, Wire::Record] {
                assert_eq!(varint_len(field_key(tag, wire)), len, "tag {tag} {wire:?}");
            }
        }
    }

    #[test]
    fn unknown_values_classified() {
        assert_eq!(unknown_value_collection(Wire::Marker, &[marker::MAP]), Some((vec![], marker::MAP)));
        // a kind a later version defines is still a marker
        assert_eq!(unknown_value_collection(Wire::Marker, &[0x99]), Some((vec![], 0x99)));
        assert_eq!(unknown_value_collection(Wire::Scalar, &[0x01]), None, "a bool is not a marker");
        assert_eq!(unknown_value_collection(Wire::Scalar, &[]), None);
        assert_eq!(unknown_value_collection(Wire::Record, &[]), None, "an empty record");
        assert_eq!(unknown_value_collection(Wire::Scalar, &7u64.to_le_bytes()), None);
        // a nested record whose tag 3 is a Lazy
        let nested = [0x04, 0x01, 0x05, 0x0D, 0x01, marker::LAZY];
        assert_eq!(unknown_value_collection(Wire::Record, &nested), Some((vec![3], marker::LAZY)));
        // ... two levels down
        let mut outer = vec![0x0A, nested.len() as u8];
        outer.extend_from_slice(&nested);
        assert_eq!(unknown_value_collection(Wire::Record, &outer), Some((vec![2, 3], marker::LAZY)));
        // a nested record of scalars only
        assert_eq!(unknown_value_collection(Wire::Record, &[0x04, 0x01, 0x05]), None);
        // a nested record that does not parse
        assert_eq!(unknown_value_collection(Wire::Record, &[0x07, 0x00]), None);
    }

    /// The round-5 false positives, now gone: a scalar under an unknown tag is never a marker,
    /// whatever its bytes. (1) a one-byte `u8` with a marker value `0x81..=0x87`; (2) a scalar
    /// whose bytes happen to parse as a record holding a marker (`09 01 81`).
    #[test]
    fn scalars_with_marker_bytes_are_not_collections() {
        for v in 0x81u8..=0x87 {
            let mut w = RecordWriter::new();
            w.push(5, vec![v]);
            let b = w.finish(None, |_| true);
            let f = parse(&b).unwrap();
            assert_eq!(f, vec![(5, Wire::Scalar, &[v][..])]);
            assert_eq!(unknown_value_collection(f[0].1, f[0].2), None, "u8 {v:#x}");
        }
        // a 3-byte scalar spelling a marker field, `09 01 81`
        let looks_like_record = [0x09u8, 0x01, marker::MAP];
        assert_eq!(parse(&looks_like_record).unwrap()[0].1, Wire::Marker, "as a record it would hold a marker");
        let mut w = RecordWriter::new();
        w.push(7, looks_like_record.to_vec());
        let b = w.finish(None, |_| true);
        let f = parse(&b).unwrap();
        assert_eq!(unknown_value_collection(f[0].1, f[0].2), None);
        // the same bytes as a nested record do hold a marker
        assert_eq!(unknown_value_collection(Wire::Record, &looks_like_record), Some((vec![2], marker::MAP)));
    }

    /// Round 7 (A3 first-write rule): the marker lands at its tag in order, nested records on
    /// the way are created or edited in place, every other field is kept byte for byte, and a
    /// marker already present leaves the record alone.
    #[test]
    fn with_marker_adds_only_a_missing_marker() {
        let mut w = RecordWriter::new();
        w.push(1, vec![7; 8]);
        w.push(9, vec![1]);
        let old = w.finish(None, |_| true);
        let new = with_marker(&old, &[8], marker::SET).unwrap().unwrap();
        assert_eq!(
            parse(&new).unwrap(),
            vec![(1, Wire::Scalar, &[7u8; 8][..]), (8, Wire::Marker, &[marker::SET][..]), (9, Wire::Scalar, &[1][..])]
        );
        assert_eq!(new.len(), old.len() + 3);
        assert_eq!(with_marker(&new, &[8], marker::SET).unwrap(), None, "already there");
        // an empty record
        assert_eq!(with_marker(&[], &[2], marker::MAP).unwrap().unwrap(), vec![0x09, 0x01, marker::MAP]);
        // a nested record that is absent is created holding just the marker
        let n = with_marker(&old, &[2, 3], marker::VECTOR).unwrap().unwrap();
        assert_eq!(find(&parse(&n).unwrap(), 2).unwrap(), &[0x0D, 0x01, marker::VECTOR]);
        assert_eq!(parse(&n).unwrap()[1].1, Wire::Record);
        // ... and edited in place when present, keeping its scalars
        let mut inner = RecordWriter::new();
        inner.push(1, vec![5]);
        let mut w = RecordWriter::new();
        w.push_record(2, inner.finish(None, |_| true));
        let outer = w.finish(None, |_| true);
        let n = with_marker(&outer, &[2, 3], marker::LAZY).unwrap().unwrap();
        assert_eq!(find(&parse(&n).unwrap(), 2).unwrap(), &[0x04, 0x01, 0x05, 0x0D, 0x01, marker::LAZY]);
        assert_eq!(with_marker(&n, &[2, 3], marker::LAZY).unwrap(), None);
        // three levels
        let n = with_marker(&[], &[1, 2, 3], marker::MAP).unwrap().unwrap();
        assert_eq!(unknown_value_collection(Wire::Record, &n), Some((vec![1, 2, 3], marker::MAP)));
        // a field on the way stored under another wire type is replaced
        let mut w = RecordWriter::new();
        w.push(2, vec![0x81]);
        let n = with_marker(&w.finish(None, |_| true), &[2], marker::MAP).unwrap().unwrap();
        assert_eq!(n, vec![0x09, 0x01, marker::MAP]);
        let mut w = RecordWriter::new();
        w.push(2, vec![0x81]);
        let n = with_marker(&w.finish(None, |_| true), &[2, 1], marker::MAP).unwrap().unwrap();
        assert_eq!(n, vec![0x0A, 0x03, 0x05, 0x01, marker::MAP]);
        // a marker of another kind at the tag counts as present (the kind byte is informative)
        assert_eq!(with_marker(&[0x09, 0x01, marker::MAP], &[2], marker::SET).unwrap(), None);
        // bytes that do not parse are an error
        assert_eq!(with_marker(&[0x07, 0x00], &[2], marker::MAP), Err(RecordError::ReservedWire { tag: 1 }));
        assert_eq!(with_marker(&[0x0A, 0x02, 0x07, 0x00], &[2, 1], marker::MAP), Err(RecordError::ReservedWire { tag: 1 }));
    }

    #[test]
    fn unknown_fields_are_merged_in_order_with_their_wire() {
        let mut w = RecordWriter::new();
        w.push(1, vec![1]);
        w.push_marker(4, marker::VECTOR);
        w.push(5, vec![5]);
        w.push_record(6, vec![0x04, 0x00]);
        let newer = w.finish(None, |_| true);
        let mut w = RecordWriter::new();
        w.push(3, vec![3]);
        w.push(1, vec![9]);
        let merged = w.finish(Some(&newer), |t| t != 1 && t != 3);
        assert_eq!(
            parse(&merged).unwrap(),
            vec![
                (1, Wire::Scalar, &[9u8][..]),
                (3, Wire::Scalar, &[3][..]),
                (4, Wire::Marker, &[marker::VECTOR][..]),
                (5, Wire::Scalar, &[5][..]),
                (6, Wire::Record, &[0x04, 0x00][..]),
            ]
        );
    }
}
