//! Tagged record encoding.
//!
//! ```text
//! record  := field*
//! field   := varint(tag) varint(len) bytes[len]
//! ```
//!
//! Canonical form, so that every record has exactly one encoding:
//! - varints are minimal LEB128 (no trailing 0x00 continuation group), at most u32 for tags/lengths;
//! - tag 0 is rejected;
//! - tags are strictly ascending (the spec does not say this; without it, field order would
//!   be a second encoding of the same record, see PROTOTYPE_NOTES.md).
//!
//! The D9 clearing flag is not in the record: a record being cleared across calls is stored at
//! its flag key (`Path::flag`, the record key plus `00`), so its bytes are the plain record.
//!
//! Kind markers (API round 5, hazard A3): a collection or `Lazy` field of a `#[near(storage)]`
//! record stores nothing inline, but leaves `varint(tag) 01 kind` in the record, one byte
//! naming its collection kind ([`marker`]). Removal reads the stored record and refuses one
//! that carries an unknown tag whose value is a marker: it is a collection some other version
//! of the contract declared, and removing the record would orphan its keys. The marker is not
//! part of any key. A root record carries none (a root is never removed).

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecordError {
    Truncated,
    NonCanonicalVarint,
    VarintOverflow,
    TagZero,
    /// Tags must be strictly ascending.
    TagOrder { prev: u32, tag: u32 },
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

/// Reads a canonical LEB128 varint that fits in u32.
pub fn read_varint_u32(bytes: &[u8], pos: &mut usize) -> Result<u32, RecordError> {
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
            break;
        }
    }
    u32::try_from(result).map_err(|_| RecordError::VarintOverflow)
}

/// Parses a record into `(tag, bytes)` pairs, enforcing the canonical form.
pub fn parse(bytes: &[u8]) -> Result<Vec<(u32, &[u8])>, RecordError> {
    let mut out = Vec::new();
    let mut pos = 0;
    let mut prev = 0u32;
    while pos < bytes.len() {
        let tag = read_varint_u32(bytes, &mut pos)?;
        if tag == 0 {
            return Err(RecordError::TagZero);
        }
        if tag <= prev {
            return Err(RecordError::TagOrder { prev, tag });
        }
        prev = tag;
        let len = read_varint_u32(bytes, &mut pos)? as usize;
        let end = pos.checked_add(len).ok_or(RecordError::Truncated)?;
        let body = bytes.get(pos..end).ok_or(RecordError::Truncated)?;
        out.push((tag, body));
        pos = end;
    }
    Ok(out)
}

/// The one-byte kind markers a collection or `Lazy` field leaves under its tag in its parent
/// record (API round 5). All are at least `0x80`, so no `bool` and no fieldless enum of fewer
/// than 129 variants can look like one; a one-byte `u8`/`i8` scalar with one of these values,
/// left behind under an unknown tag, does (removal then fails closed until the tag is listed in
/// `reserved(n)`). The values are part of the frozen record format.
pub mod marker {
    pub const MAP: u8 = 0x81;
    pub const LOOKUP_MAP: u8 = 0x82;
    pub const SET: u8 = 0x83;
    pub const LOOKUP_SET: u8 = 0x84;
    pub const VECTOR: u8 = 0x85;
    pub const LAZY: u8 = 0x86;
    pub const GROUPED_MAP: u8 = 0x87;

    /// Whether `b` is a kind marker byte.
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

/// Whether the value of a tag the code does not know is (or holds) a collection: a one-byte
/// kind marker, or a canonical nested record holding one at any depth (a nested record whose
/// own collection fields left their markers). Returns the tags below this one leading to the
/// marker, and the marker. The nested case is a heuristic: an unknown scalar whose Borsh bytes
/// happen to parse as such a record also matches, and removal then fails closed.
pub fn unknown_value_collection(body: &[u8]) -> Option<(Vec<u32>, u8)> {
    if let [b] = body {
        return marker::is_marker(*b).then(|| (Vec::new(), *b));
    }
    if body.is_empty() {
        return None;
    }
    for (t, inner) in parse(body).ok()? {
        if let Some((mut trail, k)) = unknown_value_collection(inner) {
            trail.insert(0, t);
            return Some((trail, k));
        }
    }
    None
}

/// Builds a record from known fields, then merges in unknown fields from the bytes
/// previously stored at the same key.
pub struct RecordWriter {
    fields: Vec<(u32, Vec<u8>)>,
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

    pub fn push(&mut self, tag: u32, bytes: Vec<u8>) {
        self.fields.push((tag, bytes));
    }

    /// Records that the collection field `tag` is declared: `tag 01 kind`, unless this writer
    /// is a root's.
    pub fn push_marker(&mut self, tag: u32, kind: u8) {
        debug_assert!(marker::is_marker(kind), "collection kind marker {kind:#x}");
        if self.markers {
            self.fields.push((tag, vec![kind]));
        }
    }

    /// `keep_unknown(tag)` decides whether a field found in `orig` but not pushed is preserved.
    /// The derive passes "not a known inline tag and not reserved".
    pub fn finish(mut self, orig: Option<&[u8]>, keep_unknown: impl Fn(u32) -> bool) -> Vec<u8> {
        if let Some(orig) = orig
            && let Ok(old) = parse(orig)
        {
            for (tag, body) in old {
                if keep_unknown(tag) && !self.fields.iter().any(|(t, _)| *t == tag) {
                    self.fields.push((tag, body.to_vec()));
                }
            }
        }
        self.fields.sort_by_key(|(t, _)| *t);
        for w in self.fields.windows(2) {
            assert!(w[0].0 != w[1].0, "duplicate record tag {}", w[0].0);
        }
        let mut out = Vec::new();
        for (tag, body) in &self.fields {
            write_varint(&mut out, *tag as u64);
            write_varint(&mut out, body.len() as u64);
            out.extend_from_slice(body);
        }
        out
    }
}

/// Finds the bytes of one field in a parsed record.
pub fn find<'a>(fields: &[(u32, &'a [u8])], tag: u32) -> Option<&'a [u8]> {
    fields.iter().find(|(t, _)| *t == tag).map(|(_, b)| *b)
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
    }

    /// D9 moved the clearing flag out of the record (to its own key), so tag 0 is rejected
    /// everywhere again, including the old leading `00 00`.
    #[test]
    fn tag_zero_is_rejected_even_leading_and_empty() {
        assert_eq!(parse(&[0x00, 0x00]), Err(RecordError::TagZero));
        assert_eq!(parse(&[0x00, 0x00, 0x01, 0x01, 0x07]), Err(RecordError::TagZero));
        assert_eq!(parse(&[0x01, 0x00, 0x00, 0x00]), Err(RecordError::TagZero));
    }

    #[test]
    fn tag_zero_and_order_rejected() {
        assert_eq!(parse(&[0x00, 0x01, 0x05]), Err(RecordError::TagZero));
        assert_eq!(parse(&[0x02, 0x00, 0x01, 0x00]), Err(RecordError::TagOrder { prev: 2, tag: 1 }));
        assert_eq!(parse(&[0x01, 0x00, 0x01, 0x00]), Err(RecordError::TagOrder { prev: 1, tag: 1 }));
        assert_eq!(parse(&[0x01, 0x05, 0x00]), Err(RecordError::Truncated));
    }

    /// "An Account with a nonce and a balance map is a 13-byte record": 10 for the nonce, 3 for
    /// the balance map's kind marker (API round 5; it was 10 before).
    #[test]
    fn account_record_is_thirteen_bytes() {
        let mut w = RecordWriter::new();
        w.push(1, borsh::to_vec(&7u64).unwrap());
        w.push_marker(2, marker::MAP);
        let b = w.finish(None, |_| true);
        assert_eq!(b.len(), 13);
        assert_eq!(&b[..2], &[0x01, 0x08]);
        assert_eq!(&b[10..], &[0x02, 0x01, marker::MAP]);
        // a root writes no marker
        let mut w = RecordWriter::root();
        w.push(1, borsh::to_vec(&7u64).unwrap());
        w.push_marker(2, marker::MAP);
        assert_eq!(w.finish(None, |_| true).len(), 10);
    }

    #[test]
    fn unknown_values_classified() {
        assert_eq!(unknown_value_collection(&[marker::MAP]), Some((vec![], marker::MAP)));
        assert_eq!(unknown_value_collection(&[0x01]), None, "a bool is not a marker");
        assert_eq!(unknown_value_collection(&[]), None);
        assert_eq!(unknown_value_collection(&7u64.to_le_bytes()), None);
        // a nested record whose tag 3 is a Lazy
        let nested = [0x01, 0x01, 0x05, 0x03, 0x01, marker::LAZY];
        assert_eq!(unknown_value_collection(&nested), Some((vec![3], marker::LAZY)));
        // a nested record of scalars only
        assert_eq!(unknown_value_collection(&[0x01, 0x01, 0x05]), None);
    }

    #[test]
    fn unknown_fields_are_merged_in_order() {
        let mut w = RecordWriter::new();
        w.push(1, vec![1]);
        w.push(5, vec![5]);
        let newer = w.finish(None, |_| true);
        let mut w = RecordWriter::new();
        w.push(3, vec![3]);
        w.push(1, vec![9]);
        let merged = w.finish(Some(&newer), |t| t != 1 && t != 3);
        assert_eq!(parse(&merged).unwrap(), vec![(1, &[9u8][..]), (3, &[3][..]), (5, &[5][..])]);
    }
}
