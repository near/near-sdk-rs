//! The root record and the hooks the `#[near]` wrapper calls, plus hidden helpers the
//! `#[near(storage)]` derive expands to.

use super::cache;
use super::key::{MAX_KEY_LEN, Path};
use super::record::{self, RecordError};
use super::value::{Record, Value};

/// A v2 contract root: a record at the root key.
pub trait Root: Record {
    const ROOT_BYTE: u8 = super::key::DEFAULT_ROOT;
    fn root_path() -> Path {
        Path::root(Self::ROOT_BYTE)
    }
}

/// Called before the method body: starts the call, reads the root record once, decodes it
/// and binds its collections. A missing root decodes as an empty record, so fields default
/// (and `required` fields panic).
#[track_caller]
pub fn load_root<T: Root>() -> T {
    const {
        assert!(
            1 + T::MAX_SUBPATH <= MAX_KEY_LEN,
            "this schema can produce a storage key over the 2,048-byte protocol limit"
        )
    };
    cache::begin_call();
    let p = T::root_path();
    let bytes = cache::read_root(&p);
    T::decode_value(bytes.as_deref().unwrap_or(&[]), &p)
        .unwrap_or_else(|e| crate::env::panic_str(&format!("state: cannot decode root: {e}")))
}

/// Called after a mutating method: every dirty key once in key order, then the root record
/// last, written only if its encoding changed. Returns whether the root was written.
pub fn flush_root<T: Root>(root: &T) -> bool {
    let p = T::root_path();
    cache::flush(Some((&p, &|orig| root.encode_value(orig))))
}

/// Raw tagged fields of the stored root, for a `migrate(old: RawRecord)` hook.
pub fn raw_root<T: Root>() -> Option<std::vec::Vec<(u32, std::vec::Vec<u8>)>> {
    let p = T::root_path();
    let bytes = cache::root_orig(&p).or_else(|| cache::read_root(&p))?;
    Some(record::parse(&bytes).ok()?.into_iter().map(|(t, b)| (t, b.to_vec())).collect())
}

/// Helpers the derive expands to. Not a stable API.
#[doc(hidden)]
pub mod __private {
    use super::*;
    pub use crate::state::record::{RecordWriter, find, parse};

    pub type Fields<'a> = std::vec::Vec<(u32, &'a [u8])>;

    /// Decodes field `tag`, or its `Default` (bound at `path ++ tag`) if missing.
    pub fn field<T: Value + Default>(
        fields: &Fields<'_>,
        tag: u32,
        path: &Path,
    ) -> Result<T, RecordError> {
        let fp = path.field(tag);
        match find(fields, tag) {
            Some(b) if T::INLINE => {
                T::decode_value(b, &fp).map_err(|e| RecordError::Field { tag, msg: e.to_string() })
            }
            _ => Ok(T::missing_at(&fp)),
        }
    }

    /// Decodes a `#[key(n, required)]` field: missing is an error instead of a default.
    pub fn field_required<T: Value>(
        fields: &Fields<'_>,
        tag: u32,
        path: &Path,
    ) -> Result<T, RecordError> {
        let fp = path.field(tag);
        match find(fields, tag) {
            Some(b) if T::INLINE => {
                T::decode_value(b, &fp).map_err(|e| RecordError::Field { tag, msg: e.to_string() })
            }
            _ => Err(RecordError::Missing { tag }),
        }
    }

    /// Encodes field `tag` into `w` if it is stored inline.
    pub fn encode_field<T: Value>(
        w: &mut RecordWriter,
        orig: Option<&Fields<'_>>,
        tag: u32,
        v: &T,
    ) {
        if T::INLINE {
            let o = orig.and_then(|f| find(f, tag));
            w.push(tag, v.encode_value(o));
        }
    }

    pub fn parse_orig(orig: Option<&[u8]>) -> Option<Fields<'_>> {
        orig.and_then(|o| parse(o).ok())
    }

    /// The record at `path`, read once per call, borrowed for `'a`.
    ///
    /// # Safety
    /// `'a` must be bounded by a borrow of the collection that owns `path` (see `cache.rs`).
    pub unsafe fn get_record<'a, V: Value>(path: &Path) -> Option<&'a V> {
        cache::get::<V>(path).map(|p| unsafe { &*p.as_ptr() })
    }

    pub const fn max(a: usize, b: usize) -> usize {
        if a > b { a } else { b }
    }

    /// Worst-case bytes a field adds below its record's key: nothing for an inline leaf,
    /// otherwise its tag plus its own subtree.
    pub const fn field_subpath<T: Value>(tag: u32) -> usize {
        if T::INLINE && T::MAX_SUBPATH == 0 { 0 } else { varint_len(tag) + T::MAX_SUBPATH }
    }

    pub const fn varint_len(v: u32) -> usize {
        let mut n = 1;
        let mut v = v >> 7;
        while v != 0 {
            n += 1;
            v >>= 7;
        }
        n
    }
}
