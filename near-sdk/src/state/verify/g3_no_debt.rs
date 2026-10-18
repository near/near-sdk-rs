//! Guarantee 3, No debt: host calls per operation depend only on the operation (and facts
//! about its own key), and bytes moved only on the keys and values involved, never on what
//! happened before. No free lists, no holes, no iteration that slows with age.
//!
//! The history is a random contract run through the differential driver (so the model has
//! also checked every step of it); then one operation runs in a fresh call and its receipt is
//! compared with the cost formula evaluated on facts about the operation's own key only.

use super::cost::{Cost, Facts, at_of};
use super::diff::{ops_gen, run_state};
use super::g7_stated_cost::{CostOp, formula, run_op};
use crate::state::model::{K, MState};
use crate::state::testing::{self, Receipt};

/// Read and written-or-replaced bytes for an operation on the leaf map at tag 2, from its own
/// key's value, the moved key (1 byte: `u8` keys), the 4-byte index and the header: 4 bytes
/// while the map never had a removal, 12 (`len ++ revision`) after one, absent for a map that
/// is empty and never had one (T4). A removal always writes the 12-byte form, also when the
/// map empties (the header is kept, not removed).
fn flat_bytes(op: CostOp, st: &MState) -> Option<(usize, usize)> {
    let order = st.flat();
    let val = |k: u8| st.flat.iter().find(|(x, _)| *x == k).map(|(_, v)| 4 + v.len());
    let header = crate::state::model::header_stored(order.len() as u32, st.flat_rev).map_or(0, |h| h.len());
    let header_after_insert = if st.flat_rev == 0 { 4 } else { 12 };
    Some(match op {
        CostOp::MapGet(K(k)) => (val(k).unwrap_or(0), 0),
        CostOp::MapInsert(K(k), v) => {
            let new = 4 + v.bytes().len();
            match val(k) {
                Some(old) => (4, new + old),
                // index (absent), header; entry, index, slot, header (+ replaced header)
                None => (header, new + 4 + 1 + header_after_insert + header),
            }
        }
        CostOp::MapRemove(K(k)) | CostOp::MapDiscard(K(k)) => {
            let entry = if matches!(op, CostOp::MapRemove(_)) { val(k) } else { val(k).map(|_| 0) };
            let Some(e) = entry else { return Some((if matches!(op, CostOp::MapRemove(_)) { 0 } else { 0 }, 0)) };
            let old = val(k).unwrap();
            match at_of(&order, k) {
                // entry?, index, header, last slot | moved slot, moved index, header | removes
                super::cost::At::Inner => (e + 4 + header + 1, (1 + 1) + (4 + 4) + (12 + header) + old + 4 + 1),
                // T4: the last key and the only key are the same shape now (header written)
                super::cost::At::Last | super::cost::At::Only => (e + 4 + header, (12 + header) + old + 4 + 1),
                super::cost::At::Absent => unreachable!(),
            }
        }
        _ => return None,
    })
}

fn check(history: &[crate::state::model::Op], op: CostOp) {
    let (_, st) = run_state(history);
    let Some(want) = formula(op, &st) else { return };
    testing::begin_call();
    let r: Receipt = run_op(op);
    assert_eq!(Cost::of(&r), want, "{op:?} after a history; state {st:?}\n{:#?}", r.ops);
    if let Some((rb, wb)) = flat_bytes(op, &st) {
        assert_eq!((r.read_bytes(), r.written_and_replaced_bytes()), (rb, wb), "{op:?} bytes; state {st:?}");
    }
}

/// Random engine: 5,000 histories of up to 60 operations (every collection, records with
/// items, clearing, removes), then one operation from the cost table.
#[test]
fn g3_no_debt_counts_depend_only_on_the_ops_own_key() {
    let g = (ops_gen(), bolero::produce::<CostOp>());
    bolero::check!().with_generator(g).with_iterations(5_000).for_each(|(h, op)| check(h, *op));
}

/// The spec's long-history claim: after 10,000 random operations (3 histories), every
/// operation still costs its formula. Also checked against the model at every step.
#[test]
fn g3_no_debt_after_10000_random_operations() {
    let g = (bolero::produce::<std::vec::Vec<crate::state::model::Op>>().with().len(10_000usize..=10_000), bolero::produce::<[CostOp; 8]>());
    bolero::check!().with_generator(g).with_iterations(3).for_each(|(h, ops)| {
        assert_eq!(h.len(), 10_000);
        for op in ops {
            check(h, *op);
        }
    });
}
