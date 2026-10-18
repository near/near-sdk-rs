//! Guarantee 2, Local: an operation on entry k touches its own path plus a fixed amount of
//! bookkeeping (index, header, the last key slot and the moved key's index), never another
//! entry's value. Checked on the op log after a random history, with a 100 KiB neighbour in
//! every map and in a neighbour record's map (the CI assertion the doc promises). The stated
//! exceptions (`Vector::swap_remove` moving the last element; clearing a whole record) are
//! accounted for: the vector's allowed set includes its last slot, and a record's subtree is
//! its own path.

use super::diff::{ops_gen, run_state};
use super::fixture::{MRoot, Rec};
use super::g7_stated_cost::{BIG, CostOp, NB, run_op};
use crate::state::model::{self, K, MState};
use crate::state::testing;
use crate::state::{flush_root, load_root};

/// Keys an operation on `k` may touch, given the key vectors before it.
fn allowed(op: CostOp, flat: &[u8], recs: &[u8], vlen: u32) -> Box<dyn Fn(&[u8]) -> bool> {
    let map_keys = |mk: model::spec_key::MapKeys, order: &[u8], k: u8| -> std::vec::Vec<std::vec::Vec<u8>> {
        let mut v = vec![mk.entry(k), mk.index(k), mk.header()];
        if let Some(i) = order.iter().position(|x| *x == k) {
            v.push(mk.slot(i as u32));
        }
        if let Some(last) = order.last() {
            v.push(mk.slot(order.len() as u32 - 1));
            v.push(mk.index(*last));
        }
        v.push(mk.slot(order.len() as u32)); // a new key's slot
        v
    };
    let set = |v: std::vec::Vec<std::vec::Vec<u8>>| -> Box<dyn Fn(&[u8]) -> bool> { Box::new(move |key| v.iter().any(|a| a == key)) };
    match op {
        CostOp::MapGet(K(k)) | CostOp::MapContains(K(k)) | CostOp::MapInsert(K(k), _) | CostOp::MapEntry(K(k), _)
        | CostOp::MapGetMut(K(k)) | CostOp::MapRemove(K(k)) | CostOp::MapDiscard(K(k)) => set(map_keys(model::flat(), flat, k)),
        CostOp::LookGet(K(k)) | CostOp::LookContains(K(k)) | CostOp::LookInsert(K(k), _) | CostOp::LookGetMut(K(k))
        | CostOp::LookRemove(K(k)) | CostOp::LookDiscard(K(k)) => set(vec![model::look().entry(k)]),
        CostOp::RecGet(K(k)) | CostOp::RecContains(K(k)) | CostOp::RecTryInsert(K(k)) | CostOp::RecEntryDefault(K(k))
        | CostOp::RecGetMut(K(k)) | CostOp::RecAtMutInsert(K(k), _) | CostOp::RecAtLen(K(k)) | CostOp::RecRemove(K(k))
        | CostOp::RecRemoveSome(K(k), _) => {
            let v = map_keys(model::recs(), recs, k);
            let own = model::recs().entry(k);
            Box::new(move |key| v.iter().any(|a| a == key) || key.starts_with(&own))
        }
        CostOp::VecPush(_) | CostOp::VecPop | CostOp::VecSwapRemove(_) | CostOp::VecGet(_) => {
            let vk = model::vect();
            let i = match op {
                CostOp::VecSwapRemove(i) | CostOp::VecGet(i) => i as u32,
                _ => vlen,
            };
            set(vec![vk.header(), vk.vslot(i), vk.vslot(vlen.saturating_sub(1)), vk.vslot(vlen)])
        }
        _ => Box::new(|_| true),
    }
}

/// Random engine, 3,000 cases: a history of up to 60 operations, then a 100 KiB neighbour
/// `NB` in `flat`, `look` and in neighbour record `NB`'s map (so it is the LAST key, the one
/// a removal moves), then one operation on a key in 0..5 in a fresh call. Every host call is
/// in the operation's allowed set, no call touches another entry's value key, and no call
/// moves 100 KiB.
#[test]
fn g2_local_op_touches_only_its_own_path_and_bookkeeping() {
    let g = (ops_gen(), bolero::produce::<CostOp>());
    bolero::check!().with_generator(g).with_iterations(3_000).for_each(|(h, op)| {
        if matches!(op, CostOp::SetInsert(_) | CostOp::SetContains(_) | CostOp::SetRemove(_) | CostOp::LSetInsert(_) | CostOp::LSetContains(_) | CostOp::LSetRemove(_) | CostOp::LazyGet | CostOp::LazySet(_) | CostOp::LazyRemove | CostOp::MapLen | CostOp::MapPage(..) | CostOp::MapKeysPage(..)) {
            return; // not single-entry operations
        }
        let (_, mut st): (_, MState) = run_state(h);
        // add the neighbour
        testing::begin_call();
        let mut root = load_root::<MRoot>();
        root.flat.insert(NB, vec![0xBB; BIG]);
        root.look.insert(NB, vec![0xBB; BIG]);
        if !root.recs.contains_key(&NB) {
            root.recs.try_insert(NB, Rec::default()).unwrap();
        }
        root.recs.at_mut(&NB).unwrap().items.insert(0, vec![0xBB; BIG]);
        flush_root(&root);
        drop(root);
        if !st.flat.iter().any(|(k, _)| *k == NB) {
            st.flat.push((NB, vec![]));
        }
        if !st.recs.iter().any(|(k, _)| *k == NB) {
            st.recs.push((NB, model::RecM::new(0)));
        }
        if super::g7_stated_cost::formula(*op, &st).is_none() {
            return; // would panic
        }
        let flat: std::vec::Vec<u8> = st.flat.iter().map(|(k, _)| *k).collect();
        let recs: std::vec::Vec<u8> = st.recs.iter().map(|(k, _)| *k).collect();
        let ok = allowed(*op, &flat, &recs, st.vec.len() as u32);
        let others: std::vec::Vec<std::vec::Vec<u8>> = flat
            .iter()
            .chain(recs.iter())
            .flat_map(|o| [model::flat().entry(*o), model::look().entry(*o), model::recs().entry(*o)])
            .collect();
        testing::begin_call();
        let r = run_op(*op);
        for o in &r.ops {
            assert!(ok(&o.key), "{op:?} touched {:02x?} outside its own path and bookkeeping", o.key);
            let own = match op {
                CostOp::MapGet(K(k)) | CostOp::MapContains(K(k)) | CostOp::MapInsert(K(k), _) | CostOp::MapEntry(K(k), _) | CostOp::MapGetMut(K(k)) | CostOp::MapRemove(K(k)) | CostOp::MapDiscard(K(k)) => Some(model::flat().entry(*k)),
                CostOp::LookGet(K(k)) | CostOp::LookContains(K(k)) | CostOp::LookInsert(K(k), _) | CostOp::LookGetMut(K(k)) | CostOp::LookRemove(K(k)) | CostOp::LookDiscard(K(k)) => Some(model::look().entry(*k)),
                CostOp::RecGet(K(k)) | CostOp::RecContains(K(k)) | CostOp::RecTryInsert(K(k)) | CostOp::RecEntryDefault(K(k)) | CostOp::RecGetMut(K(k)) | CostOp::RecAtMutInsert(K(k), _) | CostOp::RecAtLen(K(k)) | CostOp::RecRemove(K(k)) | CostOp::RecRemoveSome(K(k), _) => Some(model::recs().entry(*k)),
                _ => None,
            };
            if let Some(own) = own {
                assert!(o.key == own || !others.contains(&o.key), "{op:?} touched another entry's value key");
            }
        }
        if !matches!(op, CostOp::VecPop | CostOp::VecSwapRemove(_) | CostOp::VecGet(_)) {
            assert!(r.max_value_bytes() < BIG, "{op:?} moved the 100 KiB neighbour");
        }
    });
}
