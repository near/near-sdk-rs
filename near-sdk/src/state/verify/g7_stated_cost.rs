//! Guarantee 7, Stated cost: for every operation in the spec's cost table, the counting mock's
//! receipt equals the formula, with a 100 KiB entry next to the one under test. Also guarantee
//! 2's byte claim: no operation on another key moves the neighbour's 100 KiB (the stated
//! exception, `Vector::swap_remove` / `pop` reaching a large LAST element, is pinned).

use super::cost::{At, Cost, add, c, map_unlink};
use super::fixture::Rec;
use crate::state::key::{DEFAULT_ROOT, Path};
use crate::state::model::{K, Val};
use crate::state::testing::{self, Receipt};
use crate::state::{Lazy, LookupMap, LookupSet, Map, Set, Vector};
use bolero::generator::TypeGenerator;
use std::vec::Vec as StdVec;

pub const BIG: usize = 100 * 1024;
pub const NB: u8 = 9;

fn at<C: Default + crate::state::Value>(tag: u32) -> C {
    let mut c = C::default();
    c.bind(&Path::root(DEFAULT_ROOT).field(tag));
    c
}
pub fn flat() -> Map<u8, StdVec<u8>> {
    at(2)
}
pub fn look() -> LookupMap<u8, StdVec<u8>> {
    at(3)
}
pub fn recs() -> Map<u8, Rec> {
    at(4)
}
pub fn vect() -> Vector<StdVec<u8>> {
    at(5)
}
pub fn lazy() -> Lazy<StdVec<u8>> {
    at(6)
}
pub fn set() -> Set<u8> {
    at(7)
}
pub fn lset() -> LookupSet<u8> {
    at(8)
}

/// The fixture before the operation: `n` small entries (keys `0..n`) in every collection, the
/// 100 KiB (or, with `small`, 1-byte) neighbour `NB` placed first, in the middle or last, and
/// `m` items in each record (the neighbour record holds one 100 KiB item).
#[derive(Clone, Copy, Debug, TypeGenerator)]
pub struct Setup {
    #[generator(0..6)]
    pub n: u8,
    #[generator(0..3)]
    pub nb_pos: u8,
    #[generator(0..5)]
    pub m: u8,
}

impl Setup {
    /// Insertion order of the keys, with the neighbour placed.
    pub fn order(&self) -> StdVec<u8> {
        let mut v: StdVec<u8> = (0..self.n).collect();
        let at = match self.nb_pos {
            0 => 0,
            1 => v.len() / 2,
            _ => v.len(),
        };
        v.insert(at, NB);
        v
    }
    pub fn at(&self, k: u8) -> At {
        let o = self.order();
        match o.iter().position(|x| *x == k) {
            None => At::Absent,
            Some(_) if o.len() == 1 => At::Only,
            Some(i) if i == o.len() - 1 => At::Last,
            Some(_) => At::Inner,
        }
    }
    /// Seeds storage and starts a fresh call.
    pub fn seed(&self, big: usize) {
        testing::use_mock();
        let nbv = vec![0xBB; big];
        let (mut f, mut l, mut r, mut v, mut s, mut ls, mut z) = (flat(), look(), recs(), vect(), set(), lset(), lazy());
        for k in self.order() {
            let val = if k == NB { nbv.clone() } else { vec![k; 3] };
            f.insert(k, val.clone());
            l.insert(k, val.clone());
            v.push(val.clone());
            s.insert(k);
            ls.insert(k);
            let rec = r.insert_new(k, Rec { n: k as u64, ..Default::default() });
            if k == NB {
                rec.items.insert(0, nbv.clone());
            } else {
                for j in 0..self.m {
                    rec.items.insert(j, vec![j; 2]);
                }
            }
        }
        if self.n > 0 {
            z.set(vec![1, 2, 3]);
        }
        testing::end_call();
    }
}

#[derive(Clone, Copy, Debug, TypeGenerator)]
pub enum CostOp {
    MapGet(K),
    MapContains(K),
    MapInsert(K, Val),
    MapEntry(K, Val),
    MapGetMut(K),
    MapRemove(K),
    MapDiscard(K),
    MapLen,
    MapPage(Option<K>, #[generator(0..8)] u8),
    MapKeysPage(Option<K>, #[generator(0..8)] u8),
    LookGet(K),
    LookContains(K),
    LookInsert(K, Val),
    LookGetMut(K),
    LookRemove(K),
    LookDiscard(K),
    VecPush(Val),
    VecPop,
    VecSwapRemove(#[generator(0..8)] u8),
    VecGet(#[generator(0..8)] u8),
    SetInsert(K),
    SetContains(K),
    SetRemove(K),
    LSetInsert(K),
    LSetContains(K),
    LSetRemove(K),
    LazyGet,
    LazySet(Val),
    LazyRemove,
    RecGet(K),
    RecContains(K),
    RecInsertNew(K),
    RecEntryDefault(K),
    RecGetMut(K),
    RecAtMutInsert(K, K),
    RecAtLen(K),
    RecRemove(K),
    RecRemoveSome(K, #[generator(0..6)] u8),
}

/// The formula from the spec's table for `op` on `setup`, or `None` if the op would panic
/// (insert_new on an existing record, swap_remove out of bounds): those are not cost rows.
pub fn formula(op: CostOp, s: &Setup) -> Option<Cost> {
    let present = |k: u8| s.at(k) != At::Absent;
    let len = s.order().len() as u32;
    let idx = |k: u8| s.order().iter().position(|x| *x == k).map(|i| i as u32);
    let m = s.m as usize;
    Some(match op {
        CostOp::MapGet(_) | CostOp::LookGet(_) | CostOp::RecGet(_) | CostOp::VecGet(_) | CostOp::LazyGet => c(1, 0, 0, 0),
        CostOp::MapContains(_) | CostOp::LookContains(_) | CostOp::SetContains(_) | CostOp::LSetContains(_) | CostOp::RecContains(_) => c(0, 0, 0, 1),
        // new: index read, header read, 4 writes; existing: index read, 1 write
        CostOp::MapInsert(K(k), _) => if present(k) { c(1, 1, 0, 0) } else { c(2, 4, 0, 0) },
        // existing: 1 read; new: entry read, header read, 4 writes (no index read)
        CostOp::MapEntry(K(k), _) | CostOp::RecEntryDefault(K(k)) => if present(k) { c(1, 0, 0, 0) } else { c(2, 4, 0, 0) },
        CostOp::MapGetMut(K(k)) | CostOp::LookGetMut(K(k)) | CostOp::RecGetMut(K(k)) => if present(k) { c(1, 1, 0, 0) } else { c(1, 0, 0, 0) },
        CostOp::MapRemove(K(k)) => add(c(1, 0, 0, 0), map_unlink(s.at(k))),
        CostOp::MapDiscard(K(k)) | CostOp::SetRemove(K(k)) => if present(k) { map_unlink(s.at(k)) } else { c(1, 0, 0, 0) },
        CostOp::MapLen | CostOp::RecAtLen(_) => c(1, 0, 0, 0),
        // with a cursor: cursor index + header + 2 per item; without: header + 2 per item;
        // a cursor that is not in the map: CursorGone after 1 read
        CostOp::MapPage(after, limit) | CostOp::MapKeysPage(after, limit) => {
            let per = if matches!(op, CostOp::MapPage(..)) { 2 } else { 1 };
            let start = match after {
                None => 0,
                Some(K(k)) => match idx(k) {
                    None => return Some(c(1, 0, 0, 0)),
                    Some(i) => i + 1,
                },
            };
            let n = (start + limit as u32).min(len).saturating_sub(start) as usize;
            c(after.map_or(1, |_| 2) + per * n, 0, 0, 0)
        }
        CostOp::LookInsert(..) | CostOp::LSetInsert(_) | CostOp::LazySet(_) => c(0, 1, 0, 0),
        CostOp::LookRemove(K(k)) => if present(k) { c(1, 0, 1, 0) } else { c(1, 0, 0, 0) },
        CostOp::LookDiscard(_) | CostOp::LSetRemove(_) | CostOp::LazyRemove => c(0, 0, 1, 0),
        // header read, slot write, header write
        CostOp::VecPush(_) => c(1, 2, 0, 0),
        // header read, slot read, slot remove, header write (header remove when it empties)
        CostOp::VecPop => match len {
            0 => c(1, 0, 0, 0),
            1 => c(2, 0, 2, 0),
            _ => c(2, 1, 1, 0),
        },
        CostOp::VecSwapRemove(i) => {
            let i = i as u32;
            if i >= len {
                return None;
            }
            match (i == len - 1, len) {
                (true, 1) => c(2, 0, 2, 0),
                (true, _) => c(2, 1, 1, 0),
                // plus 1 read and 1 write of the last slot
                (false, _) => c(3, 2, 1, 0),
            }
        }
        CostOp::SetInsert(K(k)) => if present(k) { c(1, 0, 0, 0) } else { c(2, 4, 0, 0) },
        CostOp::RecInsertNew(K(k)) => if present(k) { return None } else { c(2, 4, 0, 0) },
        // at_mut: 1 read of the record (no decode, D9); then the items map insert:
        // new: index read, header read, 4 writes; existing: index read, 1 write
        CostOp::RecAtMutInsert(K(k), K(k2)) => {
            if !present(k) {
                c(1, 0, 0, 0)
            } else if k != NB && (k2 as usize) < m || k == NB && k2 == 0 {
                c(2, 1, 0, 0)
            } else {
                c(3, 4, 0, 0)
            }
        }
        // entry read, then clearing the record's map (header read, per item 1 slot read and 3
        // removes, the header removed if it existed), then the outer swap-remove
        CostOp::RecRemove(K(k)) => {
            if !present(k) {
                return Some(c(1, 0, 0, 0));
            }
            let m = if k == NB { 1 } else { m };
            add(c(2 + m, 0, 3 * m + (m > 0) as usize, 0), map_unlink(s.at(k)))
        }
        CostOp::RecRemoveSome(K(k), n) => {
            if !present(k) {
                return Some(c(1, 0, 0, 0));
            }
            let m = if k == NB { 1 } else { m };
            let n = n as usize;
            if n >= m {
                add(c(2 + m, 0, 3 * m + (m > 0) as usize, 0), map_unlink(s.at(k)))
            } else {
                // DOC_DIFFS[3]: one extra slot read; the record write carries the flag
                c(2 + n + 1, (n > 0) as usize + 1, 3 * n, 0)
            }
        }
    })
}

/// Runs `op` in a fresh call and returns its receipt (after the flush).
pub fn run_op(op: CostOp) -> Receipt {
    let v = |x: Val| x.bytes();
    match op {
        CostOp::MapGet(k) => drop(flat().get(&k.0).cloned()),
        CostOp::MapContains(k) => drop(flat().contains_key(&k.0)),
        CostOp::MapInsert(k, x) => drop(flat().insert(k.0, v(x))),
        CostOp::MapEntry(k, x) => drop(flat().entry(k.0).or_insert(v(x)).len()),
        CostOp::MapGetMut(k) => drop(flat().get_mut(&k.0).map(|b| b.push(1))),
        CostOp::MapRemove(k) => drop(flat().remove(&k.0)),
        CostOp::MapDiscard(k) => drop(flat().discard(&k.0)),
        CostOp::MapLen => drop(flat().len()),
        CostOp::MapPage(a, l) => drop(flat().page(a.map(|k| k.0).as_ref(), l as u32)),
        CostOp::MapKeysPage(a, l) => drop(flat().keys_page(a.map(|k| k.0).as_ref(), l as u32)),
        CostOp::LookGet(k) => drop(look().get(&k.0).cloned()),
        CostOp::LookContains(k) => drop(look().contains_key(&k.0)),
        CostOp::LookInsert(k, x) => look().insert(k.0, v(x)),
        CostOp::LookGetMut(k) => drop(look().get_mut(&k.0).map(|b| b.push(1))),
        CostOp::LookRemove(k) => drop(look().remove(&k.0)),
        CostOp::LookDiscard(k) => look().discard(&k.0),
        CostOp::VecPush(x) => vect().push(v(x)),
        CostOp::VecPop => drop(vect().pop()),
        CostOp::VecSwapRemove(i) => drop(vect().swap_remove(i as u32)),
        CostOp::VecGet(i) => drop(vect().get(i as u32).cloned()),
        CostOp::SetInsert(k) => drop(set().insert(k.0)),
        CostOp::SetContains(k) => drop(set().contains(&k.0)),
        CostOp::SetRemove(k) => drop(set().remove(&k.0)),
        CostOp::LSetInsert(k) => lset().insert(k.0),
        CostOp::LSetContains(k) => drop(lset().contains(&k.0)),
        CostOp::LSetRemove(k) => lset().remove(&k.0),
        CostOp::LazyGet => drop(lazy().get().cloned()),
        CostOp::LazySet(x) => lazy().set(v(x)),
        CostOp::LazyRemove => lazy().remove(),
        CostOp::RecGet(k) => drop(recs().get(&k.0).map(|r| r.n)),
        CostOp::RecContains(k) => drop(recs().contains_key(&k.0)),
        CostOp::RecInsertNew(k) => drop(recs().insert_new(k.0, Rec::default()).n),
        CostOp::RecEntryDefault(k) => drop(recs().entry(k.0).or_default().n),
        CostOp::RecGetMut(k) => drop(recs().get_mut(&k.0).map(|r| r.n += 1)),
        CostOp::RecAtMutInsert(k, k2) => {
            let mut r = recs();
            if let Some(a) = r.at_mut(&k.0) {
                a.items.insert(k2.0, vec![7]);
            }
        }
        CostOp::RecAtLen(k) => drop(recs().at(&k.0).items.len()),
        CostOp::RecRemove(k) => drop(recs().remove(&k.0).map(|r| r.n)),
        CostOp::RecRemoveSome(k, n) => drop(recs().remove_some(&k.0, n as u32)),
    }
    testing::end_call()
}

/// Whether `op` may legitimately move the 100 KiB neighbour (the stated exception).
fn moves_neighbour(op: CostOp, s: &Setup) -> bool {
    let o = s.order();
    let nb = o.iter().position(|x| *x == NB).unwrap() as u8;
    let last_is_big = nb as usize == o.len() - 1;
    match op {
        // the operation's own element is the neighbour
        CostOp::VecGet(i) | CostOp::VecSwapRemove(i) if i == nb => true,
        CostOp::VecPop => last_is_big,
        // a value page whose window covers the neighbour returns it
        CostOp::MapPage(after, limit) => {
            let start = match after {
                None => 0,
                Some(K(k)) => match o.iter().position(|x| *x == k) {
                    None => return false,
                    Some(i) => i + 1,
                },
            };
            (start..start + limit as usize).contains(&(nb as usize))
        }
        // the stated exception: swap_remove moves the last element into the hole
        CostOp::VecSwapRemove(_) => last_is_big,
        _ => false,
    }
}

/// G7 + G2 bytes. Random engine, 20,000 (setup, op) pairs: n <= 5 entries plus the neighbour,
/// records owning m <= 4 items, every operation in the cost table; counts must equal the
/// formula exactly. The same op beside a 1-byte neighbour must produce the same host calls
/// with the same keys and value sizes, except the stated exception.
#[test]
fn g7_stated_cost_beside_a_100kib_neighbour() {
    bolero::check!().with_type::<(Setup, CostOp)>().with_iterations(20_000).for_each(|(s, op)| {
        let Some(want) = formula(*op, s) else { return };
        s.seed(BIG);
        let r = run_op(*op);
        assert_eq!(Cost::of(&r), want, "{op:?} on {s:?} (order {:?}): {:#?}", s.order(), r.ops);
        if !moves_neighbour(*op, s) {
            assert!(r.max_value_bytes() < BIG, "{op:?} moved the 100 KiB neighbour");
            s.seed(1);
            let small = run_op(*op);
            let shape = |r: &Receipt| r.ops.iter().map(|o| (o.kind, o.key.clone(), o.value_len, o.replaced_len)).collect::<StdVec<_>>();
            assert_eq!(shape(&r), shape(&small), "{op:?}: host calls depend on the neighbour's size");
        }
    });
}

/// The stated exception, pinned: `Vector::swap_remove(0)` with a 100 KiB last element reads it
/// and writes it into slot 0.
#[test]
fn g7_swap_remove_moves_a_large_last_element_stated_exception() {
    let s = Setup { n: 2, nb_pos: 2, m: 0 };
    s.seed(BIG);
    let r = run_op(CostOp::VecSwapRemove(0));
    assert!(r.read_bytes() >= BIG && r.max_value_bytes() >= BIG);
}

/// D9, measured: `at_mut` is 1 read of the record's bytes (not a `storage_has_key`) and pays
/// the record's size. This row is in flux pending a design decision; this pins the current
/// shape: (reads, writes, removes, has_keys) = (3, 4, 0, 0) for an insert of a new key below
/// an existing record.
#[test]
fn g7_at_mut_reads_the_record_current_shape() {
    let s = Setup { n: 3, nb_pos: 0, m: 0 };
    s.seed(BIG);
    let r = run_op(CostOp::RecAtMutInsert(K(1), K(4)));
    assert_eq!(Cost::of(&r), c(3, 4, 0, 0));
    assert_eq!(r.has_keys(), 0);
    let rec_key = recs_entry_key(1);
    assert!(r.ops.iter().any(|o| o.kind == testing::OpKind::Read && o.key == rec_key));
}

fn recs_entry_key(k: u8) -> StdVec<u8> {
    crate::state::model::recs().entry(k)
}

/// sha256 per operation, measured (the spec counts one per map hop). Pinned so a change is
/// visible; printed with `--nocapture` for the report.
#[test]
fn g7_sha256_per_operation_measured() {
    let s = Setup { n: 3, nb_pos: 1, m: 2 };
    let rows: &[(&str, CostOp, u64)] = &[
        ("Map get", CostOp::MapGet(K(1)), 1),
        ("Map insert new", CostOp::MapInsert(K(4), Val { len: 1, fill: 1 }), 3),
        ("Map insert existing", CostOp::MapInsert(K(1), Val { len: 1, fill: 1 }), 2),
        ("Map remove inner", CostOp::MapRemove(K(0)), 3),
        ("Map discard inner", CostOp::MapDiscard(K(0)), 3),
        ("LookupMap insert", CostOp::LookInsert(K(1), Val { len: 1, fill: 1 }), 1),
        ("record at_mut + insert new", CostOp::RecAtMutInsert(K(1), K(4)), 4),
        ("record remove (2 items)", CostOp::RecRemove(K(0)), 7),
    ];
    for (name, op, want) in rows {
        s.seed(BIG);
        let r = run_op(*op);
        eprintln!("sha256 {name}: {}", r.sha256);
        assert_eq!(r.sha256, *want, "{name}");
    }
}
