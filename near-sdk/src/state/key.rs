//! Storage keys as paths from the root.
//!
//! ```text
//! key     := root segment*
//! root    := one byte (default 0xF0)
//! field   := LEB128(tag), tag >= 1
//! kind    := 00 entries | 01 index | 02 slots | 03 header
//! entry   := sha256(borsh(k))[..16] ++ borsh(k)
//! slot    := u32 little-endian
//! flag    := 00, once, last, after a map entry whose value owns storage
//! ```
//!
//! The flag segment (D9) names the CLEARING FLAG KEY of a record: `record_path ++ 00`. While
//! `remove_step` is clearing a record across calls, the record's bytes live at that key and
//! its own key is absent. `00` is LEB128 for tag 0, which no `#[key]` may use, and no other
//! tag's LEB128 starts with `00`, so the flag key is never a field path below the record; and
//! since map keys are prefix-free Borsh behind their own hash, it is never a sibling entry. It
//! is one byte longer than the record key and shorter than any key below the record, so it fits
//! in every schema the 2,048-byte bound accepts.
//!
//! Only [`Path`]'s builder methods can produce key bytes: there is no `Path::from_bytes`.

use borsh::{BorshDeserialize, BorshSerialize};
use near_digest::Digest;

/// Default root byte. Outside the range small `BorshStorageKey` enums and ASCII prefixes produce.
pub const DEFAULT_ROOT: u8 = 0xF0;
/// The protocol rejects storage keys longer than this.
pub const MAX_KEY_LEN: usize = 2048;
/// Width of the placement hash in front of every map key. The prototype feature
/// `unstable-state-hash8` builds an 8-byte variant, only to measure the difference in gas.
#[cfg(not(feature = "unstable-state-hash8"))]
pub const ENTRY_HASH_LEN: usize = 16;
#[cfg(feature = "unstable-state-hash8")]
pub const ENTRY_HASH_LEN: usize = 8;

/// The flag segment: appended to a record's key to form its clearing flag key (D9).
pub const FLAG_SEGMENT: u8 = 0x00;

/// Kind bytes: which internal table of a collection a key belongs to.
pub mod kind {
    pub const ENTRIES: u8 = 0x00;
    pub const INDEX: u8 = 0x01;
    pub const SLOTS: u8 = 0x02;
    pub const HEADER: u8 = 0x03;
}

/// A storage path. Owns its bytes; collections keep one per handle.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Path(Vec<u8>);

impl core::fmt::Debug for Path {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        for b in &self.0 {
            write!(f, "{b:02x}")?;
        }
        Ok(())
    }
}

impl Path {
    pub fn root(root: u8) -> Self {
        Path(vec![root])
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Field hop: append the `#[key(n)]` tag as LEB128.
    #[track_caller]
    pub fn field(&self, tag: u32) -> Path {
        assert!(tag != 0, "storage field tag 0 is reserved");
        let mut out = self.0.clone();
        crate::state::record::write_varint(&mut out, tag as u64);
        Path(out).checked()
    }

    /// Collection table hop.
    #[track_caller]
    pub fn kind(&self, kind: u8) -> Path {
        let mut out = self.0.clone();
        out.push(kind);
        Path(out).checked()
    }

    /// LookupMap entry hop: `h16(borsh(k)) ++ borsh(k)`.
    #[track_caller]
    pub fn entry<K: StorageKey + ?Sized>(&self, k: &K) -> Path {
        let enc = borsh::to_vec(k).expect("borsh serialization of a storage key cannot fail");
        assert!(
            enc.len() <= K::MAX_LEN,
            "storage key is {} bytes, over its declared MAX_LEN of {}",
            enc.len(),
            K::MAX_LEN
        );
        self.entry_raw(&enc)
    }

    /// Entry hop from already-serialized key bytes (used when a key slot is read back).
    #[track_caller]
    pub(crate) fn entry_raw(&self, enc: &[u8]) -> Path {
        let mut out = Vec::with_capacity(self.0.len() + ENTRY_HASH_LEN + enc.len());
        out.extend_from_slice(&self.0);
        out.extend_from_slice(&h16(enc));
        out.extend_from_slice(enc);
        Path(out).checked()
    }

    /// The clearing flag key of the record at this path (D9): one `00` byte appended (tag 0,
    /// which no field may use). Holds the record's bytes while `remove_step` clears it across
    /// calls; absent otherwise.
    #[track_caller]
    pub fn flag(&self) -> Path {
        let mut out = self.0.clone();
        out.push(FLAG_SEGMENT);
        Path(out).checked()
    }

    /// Dense slot hop.
    #[track_caller]
    pub fn slot(&self, i: u32) -> Path {
        let mut out = self.0.clone();
        out.extend_from_slice(&i.to_le_bytes());
        Path(out).checked()
    }

    #[track_caller]
    fn checked(self) -> Self {
        assert!(
            self.0.len() <= MAX_KEY_LEN,
            "storage key of {} bytes exceeds the protocol limit of {MAX_KEY_LEN}",
            self.0.len()
        );
        self
    }
}

std::thread_local! {
    static SHA_CALLS: core::cell::Cell<u64> = const { core::cell::Cell::new(0) };
}

/// Number of `h16` computations on this thread. Each is one `sha256` host call on chain,
/// a cost the spec's per-operation table does not list.
pub fn sha_calls() -> u64 {
    SHA_CALLS.with(|c| c.get())
}

/// First `ENTRY_HASH_LEN` bytes of sha256 (16 unless built with `unstable-state-hash8`).
pub fn h16(bytes: &[u8]) -> [u8; ENTRY_HASH_LEN] {
    SHA_CALLS.with(|c| c.set(c.get() + 1));
    let full = near_digest::sha2::Sha256::digest(bytes);
    let mut out = [0u8; ENTRY_HASH_LEN];
    out.copy_from_slice(&full[..ENTRY_HASH_LEN]);
    out
}

mod private {
    pub trait Sealed {}
}

/// A type usable as a map key. Sealed: the SDK implements it for standard types.
///
/// `MAX_LEN` is the maximum length of `borsh(k)`; it is enforced at runtime on every
/// `Path::entry` and summed along paths at compile time via `Value::MAX_SUBPATH`.
pub trait StorageKey: BorshSerialize + BorshDeserialize + private::Sealed {
    const MAX_LEN: usize;
}

macro_rules! fixed_key {
    ($($t:ty => $n:expr),* $(,)?) => {$(
        impl private::Sealed for $t {}
        impl StorageKey for $t { const MAX_LEN: usize = $n; }
    )*};
}
fixed_key!(
    u8 => 1, u16 => 2, u32 => 4, u64 => 8, u128 => 16,
    i8 => 1, i16 => 2, i32 => 4, i64 => 8, i128 => 16,
    bool => 1, () => 0,
);

/// Default cap for strings and byte vectors (borsh length prefix + 512 bytes).
pub const DEFAULT_VAR_MAX: usize = 4 + 512;

impl private::Sealed for String {}
impl StorageKey for String {
    const MAX_LEN: usize = DEFAULT_VAR_MAX;
}
impl private::Sealed for Vec<u8> {}
impl StorageKey for Vec<u8> {
    const MAX_LEN: usize = DEFAULT_VAR_MAX;
}
impl private::Sealed for crate::AccountId {}
impl StorageKey for crate::AccountId {
    const MAX_LEN: usize = 4 + 64;
}
impl<const N: usize> private::Sealed for [u8; N] {}
impl<const N: usize> StorageKey for [u8; N] {
    const MAX_LEN: usize = N;
}
impl<A: StorageKey, B: StorageKey> private::Sealed for (A, B) {}
impl<A: StorageKey, B: StorageKey> StorageKey for (A, B) {
    const MAX_LEN: usize = A::MAX_LEN + B::MAX_LEN;
}
impl<A: StorageKey> private::Sealed for Option<A> {}
impl<A: StorageKey> StorageKey for Option<A> {
    const MAX_LEN: usize = 1 + A::MAX_LEN;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::AccountId;

    fn alice() -> AccountId {
        "alice.near".parse().unwrap()
    }

    /// The spec's worked example: Exchange { #[key(3)] accounts: LookupMap<AccountId, Account> },
    /// Account { #[key(2)] balances: Map<TokenId, u128> }, TokenId = String.
    #[test]
    #[cfg(not(feature = "unstable-state-hash8"))]
    fn worked_example_lengths_and_structure() {
        let r = Path::root(DEFAULT_ROOT);
        assert_eq!(r.len(), 1);

        let acct = r.field(3).kind(kind::ENTRIES).entry(&alice());
        let b = acct.as_bytes();
        assert_eq!(b.len(), 33);
        assert_eq!(&b[..3], &[0xF0, 0x03, 0x00]);
        assert_eq!(&b[3..19], &h16(&borsh::to_vec(&alice()).unwrap()));
        assert_eq!(&b[19..23], &[0x0A, 0, 0, 0]);
        assert_eq!(&b[23..], b"alice.near");

        let balances = acct.field(2);
        let usdt = String::from("usdt");
        let val = balances.kind(kind::ENTRIES).entry(&usdt);
        assert_eq!(val.len(), 59);
        let v = val.as_bytes();
        assert_eq!(&v[..33], b);
        assert_eq!(&v[33..35], &[0x02, 0x00]);
        assert_eq!(&v[35..51], &h16(&borsh::to_vec(&usdt).unwrap()));
        assert_eq!(&v[51..55], &[4, 0, 0, 0]);
        assert_eq!(&v[55..], b"usdt");

        let idx = balances.kind(kind::INDEX).entry(&usdt);
        assert_eq!(idx.len(), 59);
        assert_eq!(&idx.as_bytes()[33..35], &[0x02, 0x01]);
        assert_eq!(&idx.as_bytes()[35..], &v[35..]);

        let slot7 = balances.kind(kind::SLOTS).slot(7);
        assert_eq!(slot7.len(), 39);
        assert_eq!(&slot7.as_bytes()[33..], &[0x02, 0x02, 7, 0, 0, 0]);

        let header = balances.kind(kind::HEADER);
        assert_eq!(header.len(), 35);
        assert_eq!(&header.as_bytes()[33..], &[0x02, 0x03]);
    }

    /// D9: the flag key is the record key plus `00`; it cannot be a field path below the
    /// record (every field tag's LEB128 starts with a nonzero byte) and sorts before them.
    #[test]
    fn flag_key_is_the_record_key_plus_a_zero_byte() {
        let rec = Path::root(DEFAULT_ROOT).field(4).kind(kind::ENTRIES).entry(&7u32);
        let flag = rec.flag();
        assert_eq!(flag.len(), rec.len() + 1);
        assert_eq!(&flag.as_bytes()[..rec.len()], rec.as_bytes());
        assert_eq!(flag.as_bytes()[rec.len()], 0x00);
        for tag in [1u32, 127, 128, 300, u32::MAX] {
            let child = rec.field(tag);
            assert_ne!(child.as_bytes()[rec.len()], 0x00, "tag {tag}");
            assert!(flag < child, "the flag key sorts before every child of the record");
        }
    }

    #[test]
    fn field_tags_are_leb128() {
        let r = Path::root(DEFAULT_ROOT);
        assert_eq!(r.field(1).as_bytes(), &[0xF0, 0x01]);
        assert_eq!(r.field(127).as_bytes(), &[0xF0, 0x7F]);
        assert_eq!(r.field(128).as_bytes(), &[0xF0, 0x80, 0x01]);
        assert_eq!(r.field(300).as_bytes(), &[0xF0, 0xAC, 0x02]);
    }

    #[test]
    #[should_panic(expected = "tag 0")]
    fn tag_zero_rejected() {
        Path::root(DEFAULT_ROOT).field(0);
    }

    #[test]
    #[should_panic(expected = "declared MAX_LEN")]
    fn oversized_string_key_panics() {
        Path::root(DEFAULT_ROOT).field(1).kind(0).entry(&"x".repeat(513));
    }

    #[test]
    fn string_key_at_cap_is_fine() {
        let p = Path::root(DEFAULT_ROOT).field(1).kind(0).entry(&"x".repeat(512));
        assert_eq!(p.len(), 3 + ENTRY_HASH_LEN + 4 + 512);
    }

    /// accounts[alice].balances[usdt] in the spec's Exchange: 59 bytes with a 16-byte hash,
    /// 43 with an 8-byte one (27 fixed bytes plus two hashes).
    #[test]
    fn nested_two_level_key_length_tracks_hash_width() {
        let v = Path::root(DEFAULT_ROOT)
            .field(3)
            .kind(kind::ENTRIES)
            .entry(&alice())
            .field(2)
            .kind(kind::ENTRIES)
            .entry(&String::from("usdt"));
        assert_eq!(v.len(), 27 + 2 * ENTRY_HASH_LEN);
        assert_eq!(v.len(), if ENTRY_HASH_LEN == 16 { 59 } else { 43 });
    }

    /// Four nested maps keyed by max-length strings exceed 2048 bytes. The runtime check
    /// catches it on the first such key; the compile-time sum (Value::MAX_SUBPATH) refuses
    /// the schema up front.
    #[test]
    #[should_panic(expected = "protocol limit")]
    fn nesting_four_long_strings_exceeds_protocol_limit() {
        let s = "x".repeat(512);
        let mut p = Path::root(DEFAULT_ROOT);
        for _ in 0..4 {
            p = p.field(1).kind(0).entry(&s);
        }
    }

    /// The hot-bridge shape: a caller-chosen 32-byte raw key in one field cannot line up
    /// with another field's entry, because the tag byte differs at position 1, before any
    /// caller-controlled byte, and the 16 bytes after the kind byte are a hash of the key.
    #[test]
    fn different_fields_diverge_before_caller_bytes() {
        let r = Path::root(DEFAULT_ROOT);
        let a = r.field(6).kind(0).entry(&[7u8; 32]);
        let b = r.field(3).kind(0).entry(&alice());
        assert_ne!(a.as_bytes()[1], b.as_bytes()[1]);
    }
}
