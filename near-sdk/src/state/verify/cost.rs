//! The spec's cost table as executable formulas, shared by guarantee 3 (no debt) and
//! guarantee 7 (stated cost). Each formula is a function of the operation and of facts about
//! the operation's OWN key only (present or not, last in the key vector or not, the map
//! emptying, the number of entries a removed record owns), never of the collection's size or
//! history.
//!
//! Source: docs/storage-v2-spec.md, "Collections and cost bounds". Where the code differs from
//! the table's wording, the formula encodes the measured behaviour and the difference is listed
//! in `DOC_DIFFS` (reported in PROTOTYPE_NOTES.md, "Soundness round").

/// Host calls of one operation in a fresh call (no root read).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct Cost {
    pub reads: usize,
    pub writes: usize,
    pub removes: usize,
    pub has_keys: usize,
}

pub const fn c(reads: usize, writes: usize, removes: usize, has_keys: usize) -> Cost {
    Cost { reads, writes, removes, has_keys }
}

impl Cost {
    pub fn of(r: &crate::state::testing::Receipt) -> Cost {
        Cost { reads: r.reads(), writes: r.writes(), removes: r.removes(), has_keys: r.has_keys() }
    }
}

/// Where the key sits in an iterable collection before the operation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum At {
    Absent,
    /// Present and not last.
    Inner,
    /// Present, last, and not the only entry.
    Last,
    /// The only entry (removing it empties the collection; since T4 a `Map` header is then
    /// written with length 0 and the bumped revision, not removed).
    Only,
}

/// `Map` / `Set` swap-remove tail after the index read, given where the key sits.
/// Spec: "1 index read, 1 header read, 2 removes for entry and index, 1 read and 1 remove of
/// the last slot, 2 writes for the moved key and its index, 1 header write (also when the map
/// empties: the header keeps the revision, T4). Removing the last key skips the moved-key
/// writes."
pub fn map_unlink(at: At) -> Cost {
    match at {
        At::Absent => c(0, 0, 0, 0),
        // index, header, last slot | moved slot, moved index, header | entry, index, last slot
        At::Inner => c(3, 3, 3, 0),
        // the last key: no moved key, so no slot read either (DOC_DIFFS[0])
        At::Last => c(2, 1, 3, 0),
        // T4: the emptied map's header is written (len 0, revision + 1), not removed
        At::Only => c(2, 1, 3, 0),
    }
}

pub fn add(a: Cost, b: Cost) -> Cost {
    Cost {
        reads: a.reads + b.reads,
        writes: a.writes + b.writes,
        removes: a.removes + b.removes,
        has_keys: a.has_keys + b.has_keys,
    }
}

/// Differences between the spec's cost wording and what the prototype measures, each pinned by
/// a formula above or in `g7_stated_cost`.
#[allow(dead_code)] // the list the report reproduces
pub const DOC_DIFFS: &[&str] = &[
    "Map/Set remove of the LAST key: the spec lists '1 read and 1 remove of the last slot' and only says the moved-key writes are skipped; the slot read is skipped too (3 reads, not 4, for remove; 2 for discard).",
    "LookupMap::discard and LookupSet::remove of an ABSENT key still issue one storage_remove (blind remove, no read); the spec does not say.",
    "Lazy::remove issues one storage_remove even when the Lazy was empty (blind remove).",
    "remove_some on a call that does not finish reads ONE MORE key slot than it removes (the loop reads the next slot before it sees the budget is spent): 2 + n + 1 reads, not 1 + n.",
    "remove_some writes the record (the clearing flag) only on the FIRST unfinished call; later unfinished calls leave identical bytes and write nothing. The spec says '1 record write for the flag on a call that does not finish'.",
    "at_mut is 1 read of the record (no decode) and no storage_has_key, as the spec now says; the rustdoc on Map::at_mut / LookupMap::at_mut still says '1 storage_has_key (no bytes)'.",
    "Vector::swap_remove(i) of a non-last element writes slot i only if the moved last element's bytes differ from the removed one's (the cache read slot i, so an identical rewrite is elided): 1 write instead of 2 then. Correct, but the spec's 'plus 1 read and 1 write of the last slot' does not say so.",
    "Map page / keys_page with limit 0 return next = None even when entries remain (a client that pages with limit 0 sees the end).",
    "sha256 per operation (spec: one per map hop): Map insert new 3, insert existing 2, remove/discard of an inner key 3 (own key twice, moved key once), at_mut + insert new below a record 4, record remove owning 2 items 7.",
];

/// What a formula may know: facts about the operation's own key, plus collection lengths
/// where the spec's formula uses them (vector bounds, page windows).
pub trait Facts {
    /// Key-vector order of the `Map<u8, Vec<u8>>` at tag 2.
    fn flat(&self) -> std::vec::Vec<u8>;
    fn look_has(&self, k: u8) -> bool;
    /// Key-vector order of the record map at tag 4.
    fn recs(&self) -> std::vec::Vec<u8>;
    /// Keys of record `k`'s items map (empty if absent).
    fn items(&self, k: u8) -> std::vec::Vec<u8>;
    /// Whether record `k`'s items map has a header key: it has entries, or had a removal
    /// (T4: an emptied map keeps its header until its record is removed).
    fn items_header(&self, k: u8) -> bool;
    fn clearing(&self, k: u8) -> bool;
    fn vec_len(&self) -> u32;
    /// Element `i` of the vector, when its bytes matter (an identical rewrite is elided).
    fn vec_at(&self, i: u32) -> Option<std::vec::Vec<u8>>;
    fn set(&self) -> std::vec::Vec<u8>;
}

pub fn at_of(order: &[u8], k: u8) -> At {
    match order.iter().position(|x| *x == k) {
        None => At::Absent,
        Some(_) if order.len() == 1 => At::Only,
        Some(i) if i == order.len() - 1 => At::Last,
        Some(_) => At::Inner,
    }
}

impl Facts for crate::state::model::MState {
    fn flat(&self) -> std::vec::Vec<u8> {
        self.flat.iter().map(|(k, _)| *k).collect()
    }
    fn look_has(&self, k: u8) -> bool {
        self.look.contains_key(&k)
    }
    fn recs(&self) -> std::vec::Vec<u8> {
        self.recs.iter().map(|(k, _)| *k).collect()
    }
    fn items(&self, k: u8) -> std::vec::Vec<u8> {
        self.recs.iter().find(|(x, _)| *x == k).map_or(vec![], |(_, r)| r.items.iter().map(|(k, _)| *k).collect())
    }
    fn items_header(&self, k: u8) -> bool {
        self.recs.iter().any(|(x, r)| *x == k && (!r.items.is_empty() || r.items_rev != 0))
    }
    fn clearing(&self, k: u8) -> bool {
        self.recs.iter().any(|(x, r)| *x == k && r.clearing)
    }
    fn vec_len(&self) -> u32 {
        self.vec.len() as u32
    }
    fn vec_at(&self, i: u32) -> Option<std::vec::Vec<u8>> {
        self.vec.get(i as usize).cloned()
    }
    fn set(&self) -> std::vec::Vec<u8> {
        vec![]
    }
}
