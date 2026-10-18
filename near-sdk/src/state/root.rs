//! The root record and the hooks the `#[near]` wrapper calls, plus hidden helpers the
//! `#[near(storage)]` derive expands to.

use super::cache;
use super::key::{MAX_KEY_LEN, Path};
use super::record::{self, RecordError};
use super::value::Value;

/// A v2 contract root: a record at the root key.
///
/// Not a `Value`: a root is never stored in a map, so it only needs encode/decode and binding.
pub trait Root: Sized {
    const ROOT_BYTE: u8 = super::key::DEFAULT_ROOT;
    const MAX_SUBPATH: usize;
    fn root_path() -> Path {
        Path::root(Self::ROOT_BYTE)
    }
    #[doc(hidden)]
    fn __encode_root(&self, orig: Option<&[u8]>) -> std::vec::Vec<u8>;
    #[doc(hidden)]
    fn __decode_root(bytes: &[u8], path: &Path) -> Result<Self, RecordError>;
    #[doc(hidden)]
    fn __bind_root(&mut self, path: &Path);
    #[doc(hidden)]
    fn __write_pending_root(&self, _path: &Path) {}

    /// Binds the root's collections to their paths. Idempotent. An `#[init]` body that seeds
    /// collections writes `let mut this = Exchange { .. }.bind();` (with `Root` in scope).
    #[track_caller]
    fn bind(mut self) -> Self {
        self.__bind_root(&Self::root_path());
        self
    }
}

/// `ContractState::state_exists` for a v2 root: `storage_has_key` through the host trait.
pub fn root_exists<T: Root>() -> bool {
    cache::host_has_key(T::root_path().as_bytes())
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
    T::__decode_root(bytes.as_deref().unwrap_or(&[]), &p)
        .unwrap_or_else(|e| crate::env::panic_str(&format!("state: cannot decode root: {e}")))
}

/// Called after a mutating method: every dirty key once in key order, then the root record
/// last, written only if its encoding changed. Returns whether the root was written.
pub fn flush_root<T: Root>(root: &T) -> bool {
    let p = T::root_path();
    // `Lazy::new` values in a root built as a struct literal (an `#[init]` body without
    // `.bind()`) are written here, so they are not lost.
    root.__write_pending_root(&p);
    cache::flush(Some((&p, &|orig| root.__encode_root(orig))))
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

    /// Decodes field `tag`, or its `Default` if missing. With a path, the value is bound at
    /// `path ++ tag`; without one (the generated `Default`), it stays unbound. The generated
    /// `decode_value` and `Default` both go through here, so a missing `Default` is reported
    /// once per field.
    pub fn field<T: Value + crate::state::FieldDefault>(
        fields: &Fields<'_>,
        tag: u32,
        path: Option<&Path>,
    ) -> Result<T, RecordError> {
        match (find(fields, tag), path) {
            (Some(b), Some(path)) if T::INLINE => T::decode_value(b, &path.field(tag))
                .map_err(|e| RecordError::Field { tag, msg: e.to_string() }),
            (_, path) => {
                let mut v = T::field_default();
                if let Some(path) = path {
                    v.bind(&path.field(tag));
                }
                Ok(v)
            }
        }
    }

    /// Decodes a `#[key(n, default = expr)]` field: missing evaluates `default` (serde-style).
    /// The type needs no `Default`. The generated `Default` for the record goes through here
    /// too, so it also uses the expression.
    pub fn field_or<T: Value>(
        fields: &Fields<'_>,
        tag: u32,
        path: Option<&Path>,
        default: impl FnOnce() -> T,
    ) -> Result<T, RecordError> {
        const {
            assert!(T::INLINE, "`#[key(n, default = ..)]` on a collection field: collections have no inline value to default")
        };
        match (find(fields, tag), path) {
            (Some(b), Some(path)) => T::decode_value(b, &path.field(tag))
                .map_err(|e| RecordError::Field { tag, msg: e.to_string() }),
            (_, path) => {
                let mut v = default();
                if let Some(path) = path {
                    v.bind(&path.field(tag));
                }
                Ok(v)
            }
        }
    }

    /// Decodes a `#[key(n, required)]` field: missing is an error instead of a default.
    pub fn field_required<T: Value>(
        fields: &Fields<'_>,
        tag: u32,
        path: Option<&Path>,
    ) -> Result<T, RecordError> {
        if !T::INLINE {
            return Err(RecordError::Leaf("`required` on a collection field".into()));
        }
        match (find(fields, tag), path) {
            (Some(b), Some(path)) => T::decode_value(b, &path.field(tag))
                .map_err(|e| RecordError::Field { tag, msg: e.to_string() }),
            _ => Err(RecordError::Missing { tag }),
        }
    }

    /// Decodes a `#[key(n, legacy)]` root field (T7): a v1 collection, or any Borsh value kept
    /// at the root, stored as its Borsh bytes. Not a `Value`; missing is an error, as for
    /// `required`.
    pub fn legacy_field<T: borsh::BorshDeserialize>(
        fields: &Fields<'_>,
        tag: u32,
    ) -> Result<T, RecordError> {
        match find(fields, tag) {
            Some(b) => borsh::from_slice(b)
                .map_err(|e| RecordError::Field { tag, msg: e.to_string() }),
            None => Err(RecordError::Missing { tag }),
        }
    }

    /// Encodes a `#[key(n, legacy)]` root field as its Borsh bytes.
    pub fn encode_legacy_field<T: borsh::BorshSerialize>(w: &mut RecordWriter, tag: u32, v: &T) {
        w.push(tag, borsh::to_vec(v).expect("borsh serialization cannot fail"));
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
