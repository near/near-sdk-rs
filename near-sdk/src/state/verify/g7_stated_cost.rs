//! Guarantee 7, Stated cost: for every operation in the spec's cost table, the counting mock's
//! receipt equals the formula, with a 100 KiB entry next to the one under test. Also guarantee
//! 2's byte claim: no operation on another key moves the neighbour's 100 KiB (the stated
//! exception, `Vector::swap_remove` / `pop` reaching a large LAST element, is pinned).

use super::cost::{Cost, Facts, add, at_of, c, map_unlink};
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
    /// Seeds storage and starts a fresh call.
    pub fn seed(&self, big: usize) {
        testing::use_mock();
        let nbv = vec![0xBB; big];
        let (mut f, mut l, mut r, mut v, mut s, mut ls, mut z) =
            (flat(), look(), recs(), vect(), set(), lset(), lazy());
        for k in self.order() {
            let val = if k == NB { nbv.clone() } else { vec![k; 3] };
            f.insert(k, val.clone());
            l.insert(k, val.clone());
            v.push(val.clone());
            s.insert(k);
            ls.insert(k);
            let rec = r.try_insert(k, Rec { n: k as u64, ..Default::default() }).unwrap();
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
    RecTryInsert(K),
    RecEntryDefault(K),
    RecGetMut(K),
    RecAtMutInsert(K, K),
    RecAtLen(K),
    RecRemove(K),
    /// API round 5: `n` is a host-remove budget, at least 3.
    RecRemoveSome(K, #[generator(3..12)] u8),
}

impl super::cost::Facts for Setup {
    fn flat(&self) -> StdVec<u8> {
        self.order()
    }
    fn look_has(&self, k: u8) -> bool {
        self.order().contains(&k)
    }
    fn recs(&self) -> StdVec<u8> {
        self.order()
    }
    fn items(&self, k: u8) -> StdVec<u8> {
        match k {
            NB => vec![0],
            k if k < self.n => (0..self.m).collect(),
            _ => vec![],
        }
    }
    fn items_header(&self, k: u8) -> bool {
        !self.items(k).is_empty()
    }
    fn clearing(&self, _: u8) -> bool {
        false
    }
    fn vec_len(&self) -> u32 {
        self.order().len() as u32
    }
    fn vec_at(&self, i: u32) -> Option<StdVec<u8>> {
        self.order().get(i as usize).map(|k| if *k == NB { vec![0xBB] } else { vec![*k; 3] })
    }
    fn set(&self) -> StdVec<u8> {
        self.order()
    }
}

/// The formula from the spec's table for `op`, from facts about its own key, or `None` if the
/// op would panic (swap_remove out of bounds, a write to or below a clearing record): those
/// are not cost rows.
///
/// D9 (API round 2): the record map's values own storage, so a record that misses at its key
/// is looked for at its flag key: a read for `get`/`get_mut`/`entry`/`remove`/`remove_step`, a
/// `storage_has_key` for `contains_key` and `at_mut`. A record being cleared lives there, so it
/// pays the same second lookup.
pub fn formula(op: CostOp, f: &impl Facts) -> Option<Cost> {
    let flat = f.flat();
    let recs = f.recs();
    let present = |k: u8| flat.contains(&k);
    let rec_present = |k: u8| recs.contains(&k);
    // record-key lookups: 1 on a live record, 2 on a miss or a record being cleared (flag key)
    let rec_lookups = |k: u8| if rec_present(k) && !f.clearing(k) { 1 } else { 2 };
    let len = flat.len() as u32;
    let vlen = f.vec_len();
    let idx = |k: u8| flat.iter().position(|x| *x == k).map(|i| i as u32);
    Some(match op {
        CostOp::MapGet(_) | CostOp::LookGet(_) | CostOp::VecGet(_) | CostOp::LazyGet => {
            c(1, 0, 0, 0)
        }
        CostOp::RecGet(K(k)) => c(rec_lookups(k), 0, 0, 0),
        CostOp::MapContains(_)
        | CostOp::LookContains(_)
        | CostOp::SetContains(_)
        | CostOp::LSetContains(_) => c(0, 0, 0, 1),
        CostOp::RecContains(K(k)) => c(0, 0, 0, rec_lookups(k)),
        // new: index read, header read, 4 writes; existing: index read, 1 write
        CostOp::MapInsert(K(k), _) => {
            if present(k) {
                c(1, 1, 0, 0)
            } else {
                c(2, 4, 0, 0)
            }
        }
        // existing: 1 read; new: entry read, header read, 4 writes (no index read)
        CostOp::MapEntry(K(k), _) => {
            if present(k) {
                c(1, 0, 0, 0)
            } else {
                c(2, 4, 0, 0)
            }
        }
        // a new record: record key, flag key, header; 4 writes
        CostOp::RecEntryDefault(K(k)) => match (rec_present(k), f.clearing(k)) {
            (true, true) => return None,
            (true, false) => c(1, 0, 0, 0),
            (false, _) => c(3, 4, 0, 0),
        },
        CostOp::MapGetMut(K(k)) => {
            if present(k) {
                c(1, 1, 0, 0)
            } else {
                c(1, 0, 0, 0)
            }
        }
        CostOp::LookGetMut(K(k)) => {
            if f.look_has(k) {
                c(1, 1, 0, 0)
            } else {
                c(1, 0, 0, 0)
            }
        }
        CostOp::RecGetMut(K(k)) => match (rec_present(k), f.clearing(k)) {
            (true, true) => return None,
            (true, false) => c(1, 1, 0, 0),
            (false, _) => c(2, 0, 0, 0),
        },
        CostOp::MapRemove(K(k)) => add(c(1, 0, 0, 0), map_unlink(at_of(&flat, k))),
        CostOp::MapDiscard(K(k)) => {
            if present(k) {
                map_unlink(at_of(&flat, k))
            } else {
                c(1, 0, 0, 0)
            }
        }
        CostOp::SetRemove(K(k)) => {
            let s = f.set();
            if s.contains(&k) { map_unlink(at_of(&s, k)) } else { c(1, 0, 0, 0) }
        }
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
        CostOp::LookRemove(K(k)) => {
            if f.look_has(k) {
                c(1, 0, 1, 0)
            } else {
                c(1, 0, 0, 0)
            }
        }
        CostOp::LookDiscard(_) | CostOp::LSetRemove(_) | CostOp::LazyRemove => c(0, 0, 1, 0),
        // header read, slot write, header write
        CostOp::VecPush(_) => c(1, 2, 0, 0),
        // header read, slot read, slot remove, header write (header remove when it empties)
        CostOp::VecPop => match vlen {
            0 => c(1, 0, 0, 0),
            1 => c(2, 0, 2, 0),
            _ => c(2, 1, 1, 0),
        },
        CostOp::VecSwapRemove(i) => {
            let i = i as u32;
            if i >= vlen {
                return None;
            }
            match (i == vlen - 1, vlen) {
                (true, 1) => c(2, 0, 2, 0),
                (true, _) => c(2, 1, 1, 0),
                // plus 1 read and 1 write of the last slot (DOC_DIFFS: the write is elided
                // when the moved element's bytes equal the removed one's)
                (false, _) => c(3, 1 + (f.vec_at(i) != f.vec_at(vlen - 1)) as usize, 1, 0),
            }
        }
        CostOp::SetInsert(K(k)) => {
            if f.set().contains(&k) {
                c(1, 0, 0, 0)
            } else {
                c(2, 4, 0, 0)
            }
        }
        // T1: `Map::try_insert` = `insert`: the index read decides; an existing record (also one
        // being cleared) is `Occupied` after that read alone
        CostOp::RecTryInsert(K(k)) => {
            if rec_present(k) {
                c(1, 0, 0, 0)
            } else {
                c(2, 4, 0, 0)
            }
        }
        // at_mut: 1 `storage_has_key` on a live record, 2 on an absent one (the flag key), no
        // read, no decode (D9); then the items map insert:
        // new: index read, header read, 4 writes; existing: index read, 1 write.
        // Round 7, the A3 first-write rule: a new key into an items map with no header (never
        // held an entry) reads the record once; its marker is there (this schema wrote it), so
        // the record is not rewritten.
        CostOp::RecAtMutInsert(K(k), K(k2)) => {
            if !rec_present(k) {
                c(0, 0, 0, 2)
            } else if f.clearing(k) {
                return None;
            } else if f.items(k).contains(&k2) {
                c(1, 1, 0, 1)
            } else if !f.items_header(k) {
                c(3, 4, 0, 1)
            } else {
                c(2, 4, 0, 1)
            }
        }
        // record read(s), then clearing the record's map (header read, per item 1 slot read and
        // 3 removes, the header removed if it exists: T4 keeps an emptied map's header), then
        // the outer swap-remove (the record's own remove is at its flag key if being cleared)
        CostOp::RecRemove(K(k)) => {
            if !rec_present(k) {
                return Some(c(2, 0, 0, 0));
            }
            let m = f.items(k).len();
            let hdr = f.items_header(k) as usize;
            add(c(rec_lookups(k) + 1 + m, 0, 3 * m + hdr, 0), map_unlink(at_of(&recs, k)))
        }
        CostOp::RecRemoveSome(K(k), n) => {
            if !rec_present(k) {
                return Some(c(2, 0, 0, 0));
            }
            // API round 5 (A5): `n` counts host removes. The FIRST call that does not finish
            // moves the record (1 remove of the record key, then 1 write of the flag key), and
            // that remove is reserved up front. An item is 3 removes, taken whole; the items
            // header (when stored) is 1 once the map is empty; finishing is 3 (record, index,
            // last key slot).
            let m = f.items(k).len();
            let hdr = f.items_header(k) as usize;
            let first = !f.clearing(k) as usize;
            let budget = n as usize - first;
            let r = m.min(budget / 3);
            let left = budget - 3 * r;
            let hdr_gone = r == m && hdr <= left;
            if hdr_gone && 3 <= left - hdr + first {
                add(c(rec_lookups(k) + 1 + m, 0, 3 * m + hdr, 0), map_unlink(at_of(&recs, k)))
            } else {
                // DOC_DIFFS[3]: with items left, one extra slot read (the next item, read
                // before its cost is known). A call that removed items but kept the header
                // writes it (revision + 1). Later unfinished calls find the record at the
                // flag key with the same bytes and write nothing (DOC_DIFFS[4]).
                c(
                    rec_lookups(k) + 1 + r + (r < m) as usize,
                    (r > 0 && !hdr_gone) as usize + first,
                    3 * r + if hdr_gone { hdr } else { 0 } + first,
                    0,
                )
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
        CostOp::RecTryInsert(k) => drop(recs().try_insert(k.0, Rec::default()).map(|r| r.n)),
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
        CostOp::RecRemoveSome(k, n) => {
            let _ = recs().remove_step(&k.0, core::num::NonZeroU32::new(n as u32).unwrap());
        }
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
            let shape = |r: &Receipt| {
                r.ops
                    .iter()
                    .map(|o| (o.kind, o.key.clone(), o.value_len, o.replaced_len))
                    .collect::<StdVec<_>>()
            };
            assert_eq!(
                shape(&r),
                shape(&small),
                "{op:?}: host calls depend on the neighbour's size"
            );
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

/// D9 after API round 2 (the flag key): `at_mut` is one existence check on the record key and
/// never reads or decodes the record, so it pays nothing for the record's size. Pinned shape
/// (reads, writes, removes, has_keys) for an insert of a new key below an existing record whose
/// items map already holds an entry: (2, 4, 0, 1), the 2 reads and 4 writes being the items
/// map's. An absent record costs a 2nd `storage_has_key`, of its flag key, and nothing else.
/// (Before the merge this pin was `g7_at_mut_reads_the_record_current_shape`, (3, 4, 0, 0) with
/// a read of the record.) Round 7: the first write into an EMPTY items map (no header) reads
/// the record once (the A3 first-write rule), (3, 4, 0, 1), and does not rewrite it (its marker
/// is there), whatever the record's size.
#[test]
fn g7_at_mut_is_one_existence_check_current_shape() {
    let s = Setup { n: 3, nb_pos: 0, m: 1 };
    s.seed(BIG);
    let r = run_op(CostOp::RecAtMutInsert(K(1), K(4)));
    assert_eq!(Cost::of(&r), c(2, 4, 0, 1));
    let rec_key = recs_entry_key(1);
    assert!(
        !r.ops.iter().any(|o| o.kind == testing::OpKind::Read && o.key == rec_key),
        "at_mut read the record"
    );
    assert!(r.ops.iter().any(|o| o.kind == testing::OpKind::HasKey && o.key == rec_key));
    let s0 = Setup { n: 3, nb_pos: 0, m: 0 };
    s0.seed(BIG);
    let r = run_op(CostOp::RecAtMutInsert(K(1), K(4)));
    assert_eq!(Cost::of(&r), c(3, 4, 0, 1));
    assert_eq!(
        r.ops.iter().filter(|o| o.kind == testing::OpKind::Read && o.key == rec_key).count(),
        1,
        "first write: 1 record read"
    );
    assert!(
        !r.ops.iter().any(|o| o.kind == testing::OpKind::Write && o.key == rec_key),
        "the marker is there: no rewrite"
    );
    s.seed(BIG);
    let r = run_op(CostOp::RecAtMutInsert(K(4), K(4)));
    let flag = crate::state::model::rec_flag(4);
    let checks: StdVec<_> = r.ops.iter().map(|o| (o.kind, o.key.clone())).collect();
    assert_eq!(
        checks,
        vec![(testing::OpKind::HasKey, recs_entry_key(4)), (testing::OpKind::HasKey, flag)]
    );
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
