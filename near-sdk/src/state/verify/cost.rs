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
    /// The only entry (removing it empties the collection and removes the header).
    Only,
}

/// `Map` / `Set` swap-remove tail after the index read, given where the key sits.
/// Spec: "1 index read, 1 header read, 2 removes for entry and index, 1 read and 1 remove of
/// the last slot, 2 writes for the moved key and its index, 1 header write. Removing the last
/// key skips the moved-key writes; emptying the map removes the header."
pub fn map_unlink(at: At) -> Cost {
    match at {
        At::Absent => c(0, 0, 0, 0),
        // index, header, last slot | moved slot, moved index, header | entry, index, last slot
        At::Inner => c(3, 3, 3, 0),
        // the last key: no moved key, so no slot read either (DOC_DIFFS[0])
        At::Last => c(2, 1, 3, 0),
        At::Only => c(2, 0, 4, 0),
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
    "Map page / keys_page with limit 0 return next = None even when entries remain (a client that pages with limit 0 sees the end).",
    "sha256 per operation (spec: one per map hop): Map insert new 3, insert existing 2, remove/discard of an inner key 3 (own key twice, moved key once), at_mut + insert new below a record 4, record remove owning 2 items 7.",
];
