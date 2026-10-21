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
//!     #[key(6)] xrecs: Map<u8, RecX>,       // Verification round 2
//!     #[key(7)] lrecs: LookupMap<u8, RecL>, // Verification round 2
//! }
//! #[near(storage)] struct Rec   { #[key(1)] n: u64, #[key(2)] items: Map<u8, Vec<u8>> }
//! // Stored as `04 08 n(8)  09 01 81`: field keys are `tag << 2 | wire`; since API round 5
//! // `items` leaves its kind marker (wire 1).
//! // Rec re-typed by a later deploy: tag 1 removed, tag 3 added with `default = 7`.
//! #[near(storage)] struct RecV2 { #[key(2)] items: Map<u8, Vec<u8>>, #[key(3, default = 7)] note: u32 }
//! // Verification round 2: records owning the other collection kinds.
//! #[near(storage)] struct RecX {          // `04 08 n(8)  09 01 86  0D 01 83  11 01 85`
//!     #[key(1)] n: u64,
//!     #[key(2)] lazy: Lazy<Vec<u8>>,
//!     #[key(3)] set: Set<u8>,
//!     #[key(4)] vect: Vector<Vec<u8>>,
//! }
//! #[near(storage)] struct RecL {          // `04 08 n(8)  09 01 83`
//!     #[key(1)] n: u64,
//!     #[key(2)] lset: Set<u8>,            // a `LookupSet` until round 7 (now root-only)
//! }
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
pub const TAG_XRECS: u32 = 6;
pub const TAG_LRECS: u32 = 7;
pub const RX_N: u32 = 1;
pub const RX_LAZY: u32 = 2;
pub const RX_SET: u32 = 3;
pub const RX_VEC: u32 = 4;
pub const RL_N: u32 = 1;
pub const RL_LSET: u32 = 2;

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
    const __RECORD: bool = true;
    fn __unknown_collection(bytes: &[u8]) -> Option<(StdVec<u32>, u8)> {
        let f = p::parse(bytes).ok()?;
        for &(t, w, b) in f.iter() {
            let hit = match t {
                REC_N => p::declared_field_collection::<u64>(w, b),
                REC_ITEMS => p::declared_field_collection::<Map<u8, StdVec<u8>>>(w, b),
                _ => p::unknown_field_collection(w, b),
            };
            if let Some((mut trail, k)) = hit {
                trail.insert(0, t);
                return Some((trail, k));
            }
        }
        None
    }
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
    fn __view_mut_in<'a>(path: Path, anchor: usize) -> RecAtMut<'a> {
        RecAtMut {
            n: <u64 as ViewFieldMut<'a, Rec>>::__view_field_mut(path.field(REC_N), anchor),
            items: <Map<u8, StdVec<u8>> as ViewFieldMut<'a, Rec>>::__view_field_mut(
                path.field(REC_ITEMS), anchor,
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
    const __RECORD: bool = true;
    fn __unknown_collection(bytes: &[u8]) -> Option<(StdVec<u32>, u8)> {
        let f = p::parse(bytes).ok()?;
        for &(t, w, b) in f.iter() {
            let hit = match t {
                REC_ITEMS => p::declared_field_collection::<Map<u8, StdVec<u8>>>(w, b),
                REC2_NOTE => p::declared_field_collection::<u32>(w, b),
                _ => p::unknown_field_collection(w, b),
            };
            if let Some((mut trail, k)) = hit {
                trail.insert(0, t);
                return Some((trail, k));
            }
        }
        None
    }
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
    fn __view_mut_in<'a>(path: Path, anchor: usize) -> RecV2AtMut<'a> {
        RecV2AtMut {
            items: <Map<u8, StdVec<u8>> as ViewFieldMut<'a, RecV2>>::__view_field_mut(
                path.field(REC_ITEMS), anchor,
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
// RecX: a record owning a Lazy, a Set and a Vector (Verification round 2)
// ---------------------------------------------------------------------------------------

#[derive(Default, Debug)]
pub struct RecX {
    pub n: u64,
    pub lazy: Lazy<StdVec<u8>>,
    pub set: Set<u8>,
    pub vect: Vector<StdVec<u8>>,
}

impl Value for RecX {
    const MAX_SUBPATH: usize = p::max(
        p::max(p::field_subpath::<u64>(RX_N), p::field_subpath::<Lazy<StdVec<u8>>>(RX_LAZY)),
        p::max(p::field_subpath::<Set<u8>>(RX_SET), p::field_subpath::<Vector<StdVec<u8>>>(RX_VEC)),
    );
    const __RECORD: bool = true;
    fn __unknown_collection(bytes: &[u8]) -> Option<(StdVec<u32>, u8)> {
        let f = p::parse(bytes).ok()?;
        for &(t, w, b) in f.iter() {
            let hit = match t {
                RX_N => p::declared_field_collection::<u64>(w, b),
                RX_LAZY => p::declared_field_collection::<Lazy<StdVec<u8>>>(w, b),
                RX_SET => p::declared_field_collection::<Set<u8>>(w, b),
                RX_VEC => p::declared_field_collection::<Vector<StdVec<u8>>>(w, b),
                _ => p::unknown_field_collection(w, b),
            };
            if let Some((mut trail, k)) = hit {
                trail.insert(0, t);
                return Some((trail, k));
            }
        }
        None
    }
    fn encode_value(&self, orig: Option<&[u8]>) -> StdVec<u8> {
        let of = p::parse_orig(orig);
        let mut w = p::RecordWriter::new();
        p::encode_field(&mut w, of.as_ref(), RX_N, &self.n);
        p::encode_field(&mut w, of.as_ref(), RX_LAZY, &self.lazy);
        p::encode_field(&mut w, of.as_ref(), RX_SET, &self.set);
        p::encode_field(&mut w, of.as_ref(), RX_VEC, &self.vect);
        w.finish(orig, |t| match t {
            RX_N => !<u64 as Value>::INLINE,
            RX_LAZY => !<Lazy<StdVec<u8>> as Value>::INLINE,
            RX_SET => !<Set<u8> as Value>::INLINE,
            RX_VEC => !<Vector<StdVec<u8>> as Value>::INLINE,
            _ => true,
        })
    }
    fn decode_value(bytes: &[u8], path: &Path) -> Result<Self, RecordError> {
        let f = p::parse(bytes)?;
        Ok(Self {
            n: p::field(&f, RX_N, Some(path))?,
            lazy: p::field(&f, RX_LAZY, Some(path))?,
            set: p::field(&f, RX_SET, Some(path))?,
            vect: p::field(&f, RX_VEC, Some(path))?,
        })
    }
    fn bind(&mut self, path: &Path) {
        Value::bind(&mut self.n, &path.field(RX_N));
        Value::bind(&mut self.lazy, &path.field(RX_LAZY));
        Value::bind(&mut self.set, &path.field(RX_SET));
        Value::bind(&mut self.vect, &path.field(RX_VEC));
    }
    fn detach(&mut self) {
        Value::detach(&mut self.n);
        Value::detach(&mut self.lazy);
        Value::detach(&mut self.set);
        Value::detach(&mut self.vect);
    }
    fn __write_pending(&self, path: &Path) {
        Value::__write_pending(&self.lazy, &path.field(RX_LAZY));
    }
}

#[allow(dead_code)]
pub struct RecXAt<'a> {
    pub n: <u64 as ViewField<'a, RecX>>::Out,
    pub lazy: <Lazy<StdVec<u8>> as ViewField<'a, RecX>>::Out,
    pub set: <Set<u8> as ViewField<'a, RecX>>::Out,
    pub vect: <Vector<StdVec<u8>> as ViewField<'a, RecX>>::Out,
    _m: PhantomData<&'a ()>,
}

#[allow(dead_code)]
pub struct RecXAtMut<'a> {
    pub n: <u64 as ViewFieldMut<'a, RecX>>::Out,
    pub lazy: <Lazy<StdVec<u8>> as ViewFieldMut<'a, RecX>>::Out,
    pub set: <Set<u8> as ViewFieldMut<'a, RecX>>::Out,
    pub vect: <Vector<StdVec<u8>> as ViewFieldMut<'a, RecX>>::Out,
    _m: PhantomData<&'a mut ()>,
}

impl Record for RecX {
    type View<'a> = RecXAt<'a>;
    type ViewMut<'a> = RecXAtMut<'a>;
    fn __view<'a>(path: Path) -> RecXAt<'a> {
        RecXAt {
            n: <u64 as ViewField<'a, RecX>>::__view_field(path.field(RX_N)),
            lazy: <Lazy<StdVec<u8>> as ViewField<'a, RecX>>::__view_field(path.field(RX_LAZY)),
            set: <Set<u8> as ViewField<'a, RecX>>::__view_field(path.field(RX_SET)),
            vect: <Vector<StdVec<u8>> as ViewField<'a, RecX>>::__view_field(path.field(RX_VEC)),
            _m: PhantomData,
        }
    }
    fn __view_mut_in<'a>(path: Path, anchor: usize) -> RecXAtMut<'a> {
        RecXAtMut {
            n: <u64 as ViewFieldMut<'a, RecX>>::__view_field_mut(path.field(RX_N), anchor),
            lazy: <Lazy<StdVec<u8>> as ViewFieldMut<'a, RecX>>::__view_field_mut(path.field(RX_LAZY), anchor),
            set: <Set<u8> as ViewFieldMut<'a, RecX>>::__view_field_mut(path.field(RX_SET), anchor),
            vect: <Vector<StdVec<u8>> as ViewFieldMut<'a, RecX>>::__view_field_mut(path.field(RX_VEC), anchor),
            _m: PhantomData,
        }
    }
}

impl Removable for RecX {
    fn clear_children(path: &Path) {
        <u64 as Removable>::clear_children(&path.field(RX_N));
        <Lazy<StdVec<u8>> as Removable>::clear_children(&path.field(RX_LAZY));
        <Set<u8> as Removable>::clear_children(&path.field(RX_SET));
        <Vector<StdVec<u8>> as Removable>::clear_children(&path.field(RX_VEC));
    }
    fn clear_children_some(path: &Path, budget: &mut u32) -> bool {
        if !<u64 as Removable>::clear_children_some(&path.field(RX_N), budget) {
            return false;
        }
        if !<Lazy<StdVec<u8>> as Removable>::clear_children_some(&path.field(RX_LAZY), budget) {
            return false;
        }
        if !<Set<u8> as Removable>::clear_children_some(&path.field(RX_SET), budget) {
            return false;
        }
        if !<Vector<StdVec<u8>> as Removable>::clear_children_some(&path.field(RX_VEC), budget) {
            return false;
        }
        true
    }
}

// ---------------------------------------------------------------------------------------
// RecL: a LookupMap value owning a Set (Verification round 2; a LookupSet until round 7,
// when headerless collections became root-only)
// ---------------------------------------------------------------------------------------

#[derive(Default, Debug)]
pub struct RecL {
    pub n: u64,
    pub lset: Set<u8>,
}

impl Value for RecL {
    const MAX_SUBPATH: usize =
        p::max(p::field_subpath::<u64>(RL_N), p::field_subpath::<Set<u8>>(RL_LSET));
    const __RECORD: bool = true;
    fn __unknown_collection(bytes: &[u8]) -> Option<(StdVec<u32>, u8)> {
        let f = p::parse(bytes).ok()?;
        for &(t, w, b) in f.iter() {
            let hit = match t {
                RL_N => p::declared_field_collection::<u64>(w, b),
                RL_LSET => p::declared_field_collection::<Set<u8>>(w, b),
                _ => p::unknown_field_collection(w, b),
            };
            if let Some((mut trail, k)) = hit {
                trail.insert(0, t);
                return Some((trail, k));
            }
        }
        None
    }
    fn encode_value(&self, orig: Option<&[u8]>) -> StdVec<u8> {
        let of = p::parse_orig(orig);
        let mut w = p::RecordWriter::new();
        p::encode_field(&mut w, of.as_ref(), RL_N, &self.n);
        p::encode_field(&mut w, of.as_ref(), RL_LSET, &self.lset);
        w.finish(orig, |t| match t {
            RL_N => !<u64 as Value>::INLINE,
            RL_LSET => !<Set<u8> as Value>::INLINE,
            _ => true,
        })
    }
    fn decode_value(bytes: &[u8], path: &Path) -> Result<Self, RecordError> {
        let f = p::parse(bytes)?;
        Ok(Self { n: p::field(&f, RL_N, Some(path))?, lset: p::field(&f, RL_LSET, Some(path))? })
    }
    fn bind(&mut self, path: &Path) {
        Value::bind(&mut self.n, &path.field(RL_N));
        Value::bind(&mut self.lset, &path.field(RL_LSET));
    }
    fn detach(&mut self) {
        Value::detach(&mut self.n);
        Value::detach(&mut self.lset);
    }
}

#[allow(dead_code)]
pub struct RecLAt<'a> {
    pub n: <u64 as ViewField<'a, RecL>>::Out,
    pub lset: <Set<u8> as ViewField<'a, RecL>>::Out,
    _m: PhantomData<&'a ()>,
}

#[allow(dead_code)]
pub struct RecLAtMut<'a> {
    pub n: <u64 as ViewFieldMut<'a, RecL>>::Out,
    pub lset: <Set<u8> as ViewFieldMut<'a, RecL>>::Out,
    _m: PhantomData<&'a mut ()>,
}

impl Record for RecL {
    type View<'a> = RecLAt<'a>;
    type ViewMut<'a> = RecLAtMut<'a>;
    fn __view<'a>(path: Path) -> RecLAt<'a> {
        RecLAt {
            n: <u64 as ViewField<'a, RecL>>::__view_field(path.field(RL_N)),
            lset: <Set<u8> as ViewField<'a, RecL>>::__view_field(path.field(RL_LSET)),
            _m: PhantomData,
        }
    }
    fn __view_mut_in<'a>(path: Path, anchor: usize) -> RecLAtMut<'a> {
        RecLAtMut {
            n: <u64 as ViewFieldMut<'a, RecL>>::__view_field_mut(path.field(RL_N), anchor),
            lset: <Set<u8> as ViewFieldMut<'a, RecL>>::__view_field_mut(path.field(RL_LSET), anchor),
            _m: PhantomData,
        }
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
    pub xrecs: Map<u8, RecX>,
    pub lrecs: LookupMap<u8, RecL>,
}

impl Root for MRoot {
    const MAX_SUBPATH: usize = p::max(
        p::max(
            p::max(p::field_subpath::<u64>(TAG_COUNTER), p::field_subpath::<Map<u8, StdVec<u8>>>(TAG_FLAT)),
            p::max(
                p::max(
                    p::field_subpath::<LookupMap<u8, StdVec<u8>>>(TAG_LOOK),
                    p::field_subpath::<Map<u8, Rec>>(TAG_RECS),
                ),
                p::field_subpath::<Vector<StdVec<u8>>>(TAG_VEC),
            ),
        ),
        p::max(p::field_subpath::<Map<u8, RecX>>(TAG_XRECS), p::field_subpath::<LookupMap<u8, RecL>>(TAG_LRECS)),
    );
    fn __encode_root(&self, orig: Option<&[u8]>) -> StdVec<u8> {
        let of = p::parse_orig(orig);
        let mut w = p::RecordWriter::root();
        p::encode_field(&mut w, of.as_ref(), TAG_COUNTER, &self.counter);
        p::encode_field(&mut w, of.as_ref(), TAG_FLAT, &self.flat);
        p::encode_field(&mut w, of.as_ref(), TAG_LOOK, &self.look);
        p::encode_field(&mut w, of.as_ref(), TAG_RECS, &self.recs);
        p::encode_field(&mut w, of.as_ref(), TAG_VEC, &self.vec);
        p::encode_field(&mut w, of.as_ref(), TAG_XRECS, &self.xrecs);
        p::encode_field(&mut w, of.as_ref(), TAG_LRECS, &self.lrecs);
        w.finish(orig, |t| match t {
            TAG_COUNTER => !<u64 as Value>::INLINE,
            TAG_FLAT => !<Map<u8, StdVec<u8>> as Value>::INLINE,
            TAG_LOOK => !<LookupMap<u8, StdVec<u8>> as Value>::INLINE,
            TAG_RECS => !<Map<u8, Rec> as Value>::INLINE,
            TAG_VEC => !<Vector<StdVec<u8>> as Value>::INLINE,
            TAG_XRECS => !<Map<u8, RecX> as Value>::INLINE,
            TAG_LRECS => !<LookupMap<u8, RecL> as Value>::INLINE,
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
            xrecs: p::field(&f, TAG_XRECS, Some(path))?,
            lrecs: p::field(&f, TAG_LRECS, Some(path))?,
        })
    }
    fn __bind_root(&mut self, path: &Path) {
        Value::bind(&mut self.counter, &path.field(TAG_COUNTER));
        Value::bind(&mut self.flat, &path.field(TAG_FLAT));
        Value::bind(&mut self.look, &path.field(TAG_LOOK));
        Value::bind(&mut self.recs, &path.field(TAG_RECS));
        Value::bind(&mut self.vec, &path.field(TAG_VEC));
        Value::bind(&mut self.xrecs, &path.field(TAG_XRECS));
        Value::bind(&mut self.lrecs, &path.field(TAG_LRECS));
    }
}

/// `Map<u8, RecV2>` over the same path as `MRoot::recs`: the record map as a later deploy
/// sees it. Only touched in a call of its own (the cache refuses two types for one key).
pub fn recs_v2() -> Map<u8, RecV2> {
    Map::__bound_at(Path::root(crate::state::key::DEFAULT_ROOT).field(TAG_RECS))
}
