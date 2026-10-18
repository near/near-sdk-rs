//! Guarantee 5, Linear iteration: a page of n entries costs two reads plus two per entry (one
//! per entry for keys alone), whatever the collection's size or age; a cursor whose key was
//! removed gives `CursorGone`.

use super::g7_stated_cost::flat;
use crate::state::CursorGone;
use crate::state::testing;
use bolero::generator::TypeGenerator;

#[derive(Clone, Debug, TypeGenerator)]
pub struct Scenario {
    /// Keys `0..size` inserted in order.
    #[generator(0..=200)]
    pub size: u8,
    /// Keys removed afterwards (in this order), as raw bytes reduced mod `size`.
    #[generator(bolero::produce::<std::vec::Vec<u8>>().with().len(0usize..=80))]
    pub removals: std::vec::Vec<u8>,
    /// Calls the history is split into.
    #[generator(1..=4)]
    pub calls: u8,
    /// Cursor: `None`, a key still present, or a removed key.
    pub cursor: Option<u8>,
    #[generator(0..=64)]
    pub limit: u8,
    pub keys_only: bool,
}

impl Scenario {
    /// Builds the map and returns the expected key vector (swap-remove order).
    fn build(&self) -> (std::vec::Vec<u8>, std::vec::Vec<u8>) {
        testing::use_mock();
        let mut m = flat();
        let mut order: std::vec::Vec<u8> = (0..self.size).collect();
        let mut removed = std::vec::Vec::new();
        for k in 0..self.size {
            m.insert(k, vec![k; (k % 7) as usize]);
        }
        let per_call = (self.removals.len() / self.calls as usize).max(1);
        for (j, r) in self.removals.iter().enumerate() {
            if self.size == 0 {
                break;
            }
            if j % per_call == 0 {
                testing::end_call();
                m = flat();
            }
            let k = r % self.size;
            if let Some(i) = order.iter().position(|x| *x == k) {
                assert!(m.remove(&k).is_some());
                order.swap_remove(i);
                removed.push(k);
            } else {
                assert!(m.remove(&k).is_none());
            }
        }
        testing::end_call();
        (order, removed)
    }
}

/// Random engine, 5,000 scenarios: maps of 0..=200 keys after up to 80 removals spread over
/// 1..=4 calls, pages of limit 0..=64 from no cursor, a live cursor or a removed cursor.
/// Reads are exactly 2 + 2n (keys only: 2 + n) with a cursor, 1 + 2n (1 + n) without, where
/// n is the number of items returned; the items are the key vector's, in order, with their
/// current values; a removed cursor is `CursorGone` after 1 read.
#[test]
fn g5_page_costs_two_reads_plus_two_per_entry() {
    bolero::check!().with_type::<Scenario>().with_iterations(5_000).for_each(|s| {
        let (order, removed) = s.build();
        let cursor = s.cursor.map(|c| {
            if c % 2 == 0 && !order.is_empty() {
                order[c as usize % order.len()]
            } else if !removed.is_empty() {
                removed[c as usize % removed.len()]
            } else {
                c
            }
        });
        let m = flat();
        let r = if s.keys_only {
            m.keys_page(cursor.as_ref(), s.limit as u32).map(|p| (p.keys, p.next))
        } else {
            m.page(cursor.as_ref(), s.limit as u32).map(|p| {
                for (k, v) in &p.items {
                    assert_eq!(v, &vec![*k; (k % 7) as usize], "wrong value for key {k}");
                }
                (p.items.into_iter().map(|(k, _)| k).collect(), p.next)
            })
        };
        let rc = testing::end_call();
        let start = match cursor {
            None => Some(0),
            Some(c) => order.iter().position(|x| *x == c).map(|i| i + 1),
        };
        match (start, r) {
            (None, Err(CursorGone)) => assert_eq!(rc.reads(), 1, "CursorGone costs 1 read"),
            (None, Ok(_)) => panic!("a removed cursor returned a page"),
            (Some(_), Err(_)) => panic!("a live cursor was CursorGone"),
            (Some(st), Ok((keys, next))) => {
                let end = (st + s.limit as usize).min(order.len());
                let want: std::vec::Vec<u8> = order[st.min(order.len())..end].to_vec();
                assert_eq!(keys, want);
                let n = keys.len();
                let per = if s.keys_only { 1 } else { 2 };
                let base = if cursor.is_some() { 2 } else { 1 };
                assert_eq!(rc.reads(), base + per * n, "page of {n} on a map of {}", order.len());
                assert_eq!(rc.writes() + rc.removes() + rc.has_keys(), 0);
                let want_next = if end < order.len() { keys.last().copied() } else { None };
                assert_eq!(next, want_next);
            }
        }
    });
}

/// The spec's harness bound, explicitly: n up to 64 on a map of 200 after 100 removals.
#[test]
fn g5_page_of_64_on_an_aged_map() {
    let s = Scenario { size: 200, removals: (0..100).map(|i| (i * 37 % 200) as u8).collect(), calls: 4, cursor: None, limit: 64, keys_only: false };
    let (order, _) = s.build();
    let m = flat();
    let p = m.page(Some(&order[10]), 64).unwrap();
    assert_eq!(p.items.len(), 64.min(order.len() - 11));
    let r = testing::end_call();
    assert_eq!(r.reads(), 2 + 2 * p.items.len());
}
