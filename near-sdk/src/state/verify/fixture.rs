//! The schema every model-driven harness runs against. HAND-EXPANDED, like `state::tests`:
//! exactly what `#[near(storage)]` / `contract_state(layout = v2)` emit (see
//! near-sdk-macros/src/core_impl/code_generator/storage_v2.rs), because `::near_sdk` paths do
//! not resolve inside the crate. The derive itself is exercised by `tests/state_v2_verify.rs`.
//!
//! ```text
//! #[near(contract_state(layout = v2))]
//! struct MRoot {
//!     #[key(1)] counter: u64,
//!     #[key(2)] flat: Map<u8, Vec<u8>>,
//!     #[key(3)] look: LookupMap<u8, Vec<u8>>,
//!     #[key(4)] recs: Map<u8, Rec>,
//!     #[key(5)] vec: Vector<Vec<u8>>,
//! }
//! #[near(storage)] struct Rec   { #[key(1)] n: u64, #[key(2)] items: Map<u8, Vec<u8>> }
//! // Rec re-typed by a later deploy: tag 1 removed, tag 3 added with `default = 7`.
//! #[near(storage)] struct RecV2 { #[key(2)] items: Map<u8, Vec<u8>>, #[key(3, default = 7)] note: u32 }
//! ```

use crate::state::key::Path;
use crate::state::record::RecordError;
use crate::state::root::__private as p;
use crate::state::*;
use core::marker::PhantomData;
use std::vec::Vec as StdVec;

pub const TAG_COUNTER: u32 = 1;
pub const TAG_FLAT: u32 = 2;
pub const TAG_LOOK: u32 = 3;
pub const TAG_RECS: u32 = 4;
pub const TAG_VEC: u32 = 5;
pub const REC_N: u32 = 1;
pub const REC_ITEMS: u32 = 2;
pub const REC2_NOTE: u32 = 3;
pub const REC2_NOTE_DEFAULT: u32 = 7;

// ---------------------------------------------------------------------------------------
// Rec
// ---------------------------------------------------------------------------------------

#[derive(Default, Debug)]
pub struct Rec {
    pub n: u64,
    pub items: Map<u8, StdVec<u8>>,
}

impl Value for Rec {
    const MAX_SUBPATH: usize = p::max(
        p::field_subpath::<u64>(REC_N),
        p::field_subpath::<Map<u8, StdVec<u8>>>(REC_ITEMS),
    );
    fn encode_value(&self, orig: Option<&[u8]>) -> StdVec<u8> {
        let of = p::parse_orig(orig);
        let mut w = p::RecordWriter::new();
        p::encode_field(&mut w, of.as_ref(), REC_N, &self.n);
        p::encode_field(&mut w, of.as_ref(), REC_ITEMS, &self.items);
        w.finish(orig, |t| match t {
            REC_N => !<u64 as Value>::INLINE,
            REC_ITEMS => !<Map<u8, StdVec<u8>> as Value>::INLINE,
            _ => true,
        })
    }
    fn decode_value(bytes: &[u8], path: &Path) -> Result<Self, RecordError> {
        let f = p::parse(bytes)?;
        Ok(Self { n: p::field(&f, REC_N, Some(path))?, items: p::field(&f, REC_ITEMS, Some(path))? })
    }
    fn bind(&mut self, path: &Path) {
        Value::bind(&mut self.n, &path.field(REC_N));
        Value::bind(&mut self.items, &path.field(REC_ITEMS));
    }
    fn detach(&mut self) {
        Value::detach(&mut self.n);
        Value::detach(&mut self.items);
    }
}

#[allow(dead_code)]
pub struct RecAt<'a> {
    pub n: <u64 as ViewField<'a, Rec>>::Out,
    pub items: <Map<u8, StdVec<u8>> as ViewField<'a, Rec>>::Out,
    _m: PhantomData<&'a ()>,
}

#[allow(dead_code)]
pub struct RecAtMut<'a> {
    pub n: <u64 as ViewFieldMut<'a, Rec>>::Out,
    pub items: <Map<u8, StdVec<u8>> as ViewFieldMut<'a, Rec>>::Out,
    _m: PhantomData<&'a mut ()>,
}

impl Record for Rec {
    type View<'a> = RecAt<'a>;
    type ViewMut<'a> = RecAtMut<'a>;
    fn __view<'a>(path: Path) -> RecAt<'a> {
        RecAt {
            n: <u64 as ViewField<'a, Rec>>::__view_field(path.field(REC_N)),
            items: <Map<u8, StdVec<u8>> as ViewField<'a, Rec>>::__view_field(path.field(REC_ITEMS)),
            _m: PhantomData,
        }
    }
    fn __view_mut<'a>(path: Path) -> RecAtMut<'a> {
        RecAtMut {
            n: <u64 as ViewFieldMut<'a, Rec>>::__view_field_mut(path.field(REC_N)),
            items: <Map<u8, StdVec<u8>> as ViewFieldMut<'a, Rec>>::__view_field_mut(
                path.field(REC_ITEMS),
            ),
            _m: PhantomData,
        }
    }
}

impl Removable for Rec {
    fn clear_children(path: &Path) {
        <u64 as Removable>::clear_children(&path.field(REC_N));
        <Map<u8, StdVec<u8>> as Removable>::clear_children(&path.field(REC_ITEMS));
    }
    fn clear_children_some(path: &Path, budget: &mut u32) -> bool {
        if !<u64 as Removable>::clear_children_some(&path.field(REC_N), budget) {
            return false;
        }
        if !<Map<u8, StdVec<u8>> as Removable>::clear_children_some(&path.field(REC_ITEMS), budget) {
            return false;
        }
        true
    }
}

// ---------------------------------------------------------------------------------------
// RecV2: Rec after a deploy that removed tag 1 and added tag 3 with `default = 7`.
// ---------------------------------------------------------------------------------------

#[derive(Debug)]
pub struct RecV2 {
    pub items: Map<u8, StdVec<u8>>,
    pub note: u32,
}

impl Default for RecV2 {
    fn default() -> Self {
        let f: p::Fields<'_> = StdVec::new();
        Self {
            items: p::field(&f, REC_ITEMS, None).expect("default"),
            note: p::field_or(&f, REC2_NOTE, None, || REC2_NOTE_DEFAULT).expect("default"),
        }
    }
}

impl Value for RecV2 {
    const MAX_SUBPATH: usize = p::max(
        p::field_subpath::<Map<u8, StdVec<u8>>>(REC_ITEMS),
        p::field_subpath::<u32>(REC2_NOTE),
    );
    fn encode_value(&self, orig: Option<&[u8]>) -> StdVec<u8> {
        let of = p::parse_orig(orig);
        let mut w = p::RecordWriter::new();
        p::encode_field(&mut w, of.as_ref(), REC_ITEMS, &self.items);
        p::encode_field(&mut w, of.as_ref(), REC2_NOTE, &self.note);
        w.finish(orig, |t| match t {
            REC_ITEMS => !<Map<u8, StdVec<u8>> as Value>::INLINE,
            REC2_NOTE => !<u32 as Value>::INLINE,
            _ => true,
        })
    }
    fn decode_value(bytes: &[u8], path: &Path) -> Result<Self, RecordError> {
        let f = p::parse(bytes)?;
        Ok(Self {
            items: p::field(&f, REC_ITEMS, Some(path))?,
            note: p::field_or(&f, REC2_NOTE, Some(path), || REC2_NOTE_DEFAULT)?,
        })
    }
    fn bind(&mut self, path: &Path) {
        Value::bind(&mut self.items, &path.field(REC_ITEMS));
        Value::bind(&mut self.note, &path.field(REC2_NOTE));
    }
    fn detach(&mut self) {
        Value::detach(&mut self.items);
        Value::detach(&mut self.note);
    }
}

#[allow(dead_code)]
pub struct RecV2At<'a> {
    pub items: <Map<u8, StdVec<u8>> as ViewField<'a, RecV2>>::Out,
    _m: PhantomData<&'a ()>,
}
#[allow(dead_code)]
pub struct RecV2AtMut<'a> {
    pub items: <Map<u8, StdVec<u8>> as ViewFieldMut<'a, RecV2>>::Out,
    _m: PhantomData<&'a mut ()>,
}
impl Record for RecV2 {
    type View<'a> = RecV2At<'a>;
    type ViewMut<'a> = RecV2AtMut<'a>;
    fn __view<'a>(path: Path) -> RecV2At<'a> {
        RecV2At {
            items: <Map<u8, StdVec<u8>> as ViewField<'a, RecV2>>::__view_field(path.field(REC_ITEMS)),
            _m: PhantomData,
        }
    }
    fn __view_mut<'a>(path: Path) -> RecV2AtMut<'a> {
        RecV2AtMut {
            items: <Map<u8, StdVec<u8>> as ViewFieldMut<'a, RecV2>>::__view_field_mut(
                path.field(REC_ITEMS),
            ),
            _m: PhantomData,
        }
    }
}
impl Removable for RecV2 {
    fn clear_children(path: &Path) {
        <Map<u8, StdVec<u8>> as Removable>::clear_children(&path.field(REC_ITEMS));
    }
    fn clear_children_some(path: &Path, budget: &mut u32) -> bool {
        <Map<u8, StdVec<u8>> as Removable>::clear_children_some(&path.field(REC_ITEMS), budget)
    }
}

// ---------------------------------------------------------------------------------------
// MRoot
// ---------------------------------------------------------------------------------------

pub struct MRoot {
    pub counter: u64,
    pub flat: Map<u8, StdVec<u8>>,
    pub look: LookupMap<u8, StdVec<u8>>,
    pub recs: Map<u8, Rec>,
    pub vec: Vector<StdVec<u8>>,
}

impl Root for MRoot {
    const MAX_SUBPATH: usize = p::max(
        p::max(p::field_subpath::<u64>(TAG_COUNTER), p::field_subpath::<Map<u8, StdVec<u8>>>(TAG_FLAT)),
        p::max(
            p::max(
                p::field_subpath::<LookupMap<u8, StdVec<u8>>>(TAG_LOOK),
                p::field_subpath::<Map<u8, Rec>>(TAG_RECS),
            ),
            p::field_subpath::<Vector<StdVec<u8>>>(TAG_VEC),
        ),
    );
    fn __encode_root(&self, orig: Option<&[u8]>) -> StdVec<u8> {
        let of = p::parse_orig(orig);
        let mut w = p::RecordWriter::new();
        p::encode_field(&mut w, of.as_ref(), TAG_COUNTER, &self.counter);
        p::encode_field(&mut w, of.as_ref(), TAG_FLAT, &self.flat);
        p::encode_field(&mut w, of.as_ref(), TAG_LOOK, &self.look);
        p::encode_field(&mut w, of.as_ref(), TAG_RECS, &self.recs);
        p::encode_field(&mut w, of.as_ref(), TAG_VEC, &self.vec);
        w.finish(orig, |t| match t {
            TAG_COUNTER => !<u64 as Value>::INLINE,
            TAG_FLAT => !<Map<u8, StdVec<u8>> as Value>::INLINE,
            TAG_LOOK => !<LookupMap<u8, StdVec<u8>> as Value>::INLINE,
            TAG_RECS => !<Map<u8, Rec> as Value>::INLINE,
            TAG_VEC => !<Vector<StdVec<u8>> as Value>::INLINE,
            _ => true,
        })
    }
    fn __decode_root(bytes: &[u8], path: &Path) -> Result<Self, RecordError> {
        let f = p::parse(bytes)?;
        Ok(Self {
            counter: p::field(&f, TAG_COUNTER, Some(path))?,
            flat: p::field(&f, TAG_FLAT, Some(path))?,
            look: p::field(&f, TAG_LOOK, Some(path))?,
            recs: p::field(&f, TAG_RECS, Some(path))?,
            vec: p::field(&f, TAG_VEC, Some(path))?,
        })
    }
    fn __bind_root(&mut self, path: &Path) {
        Value::bind(&mut self.counter, &path.field(TAG_COUNTER));
        Value::bind(&mut self.flat, &path.field(TAG_FLAT));
        Value::bind(&mut self.look, &path.field(TAG_LOOK));
        Value::bind(&mut self.recs, &path.field(TAG_RECS));
        Value::bind(&mut self.vec, &path.field(TAG_VEC));
    }
}

/// `Map<u8, RecV2>` over the same path as `MRoot::recs`: the record map as a later deploy
/// sees it. Only touched in a call of its own (the cache refuses two types for one key).
pub fn recs_v2() -> Map<u8, RecV2> {
    Map::__bound_at(Path::root(crate::state::key::DEFAULT_ROOT).field(TAG_RECS))
}
