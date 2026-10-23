//! Bug class C6, paging under churn. A scan pages a map in one call per page while other calls
//! insert, remove and update between pages. Against an exact mirror of the key vector
//! (swap-remove order), every page is either `Err(CursorGone)` (exactly when the cursor key was
//! removed) or exactly the mirror's next window with CURRENT values; offset pages never fail
//! and are exactly the window at the offset. Over a whole scan: offset paging never returns a
//! key twice; CURSOR paging can (a swap-remove can move the cursor key itself toward the
//! front, so the next page re-reads keys already returned; pinned below, and not in the
//! `Map::page` rustdoc, which only names the skip); with insert-only churn every key present at
//! the start is returned exactly once in both modes; a key present throughout the scan is
//! skipped only if a removal moved it (the stated skip).

use super::g7_stated_cost::flat;
use crate::state::CursorGone;
use crate::state::testing;
use bolero::generator::TypeGenerator;

#[derive(Clone, Copy, Debug, TypeGenerator)]
pub enum Churn {
    Insert,
    Remove(u8),
    Update(u8),
    Nothing,
}

#[derive(Clone, Debug, TypeGenerator)]
pub struct Scan {
    #[generator(0..=40)]
    pub size: u8,
    #[generator(1..=8)]
    pub limit: u8,
    pub offset_mode: bool,
    pub insert_only: bool,
    #[generator(bolero::produce::<Vec<[Churn; 2]>>().with().len(0usize..=30))]
    pub churn: Vec<[Churn; 2]>,
}

fn val(k: u8, ver: u8) -> Vec<u8> {
    vec![k, ver]
}

/// Random engine, 5,000 scans of maps of 0..=40 keys, pages of 1..=8, up to 2 churn ops
/// between pages.
#[test]
fn c6_paging_under_churn_is_exact_or_cursor_gone() {
    bolero::check!().with_type::<Scan>().with_iterations(5_000).for_each(|s| {
        testing::use_mock();
        let mut m = flat();
        let mut order: Vec<u8> = (0..s.size).collect();
        let mut ver = [0u8; 256];
        for k in 0..s.size {
            m.insert(k, val(k, 0));
        }
        testing::end_call();
        let initial = order.clone();
        let mut next_new = 100u8;
        let mut moved: Vec<u8> = Vec::new();
        let mut removed: Vec<u8> = Vec::new();
        let mut seen: Vec<u8> = Vec::new();
        let mut cursor: Option<u8> = None;
        let mut offset = 0usize;
        let mut gone = false;
        let mut step = 0usize;
        loop {
            // one page, in its own call
            let m = flat();
            let (keys, done) = if s.offset_mode {
                let items: Vec<(u8, Vec<u8>)> = m
                    .iter()
                    .skip(offset)
                    .take(s.limit as usize)
                    .map(|(k, v)| (k, v.clone()))
                    .collect();
                let want: Vec<u8> = order
                    [offset.min(order.len())..(offset + s.limit as usize).min(order.len())]
                    .to_vec();
                assert_eq!(
                    items.iter().map(|(k, _)| *k).collect::<Vec<_>>(),
                    want,
                    "offset page differs from the window"
                );
                for (k, v) in &items {
                    assert_eq!(v, &val(*k, ver[*k as usize]), "stale value for {k}");
                }
                offset += items.len();
                let done = items.len() < s.limit as usize;
                (want, done)
            } else {
                let start = match cursor {
                    None => Some(0),
                    Some(c) => order.iter().position(|x| *x == c).map(|i| i + 1),
                };
                match (m.page(cursor.as_ref(), s.limit as u32), start) {
                    (Err(CursorGone), None) => {
                        gone = true;
                        (vec![], true)
                    }
                    (Err(CursorGone), Some(_)) => panic!("a live cursor was CursorGone"),
                    (Ok(_), None) => panic!("a removed cursor returned a page"),
                    (Ok(p), Some(st)) => {
                        let end = (st + s.limit as usize).min(order.len());
                        let want = order[st.min(order.len())..end].to_vec();
                        assert_eq!(
                            p.items.iter().map(|(k, _)| *k).collect::<Vec<_>>(),
                            want,
                            "cursor page differs from the window"
                        );
                        for (k, v) in &p.items {
                            assert_eq!(v, &val(*k, ver[*k as usize]), "stale value for {k}");
                        }
                        assert_eq!(
                            p.next,
                            if end < order.len() { want.last().copied() } else { None }
                        );
                        cursor = p.next;
                        (want, p.next.is_none())
                    }
                }
            };
            testing::end_call();
            seen.extend(keys);
            if done {
                break;
            }
            // churn, in its own call
            let mut m = flat();
            for c in s.churn.get(step).copied().unwrap_or([Churn::Nothing; 2]) {
                match c {
                    Churn::Insert if next_new < 250 => {
                        m.insert(next_new, val(next_new, 0));
                        order.push(next_new);
                        next_new += 1;
                    }
                    Churn::Remove(i) if !s.insert_only && !order.is_empty() => {
                        let i = i as usize % order.len();
                        let k = order.swap_remove(i);
                        if i < order.len() {
                            moved.push(order[i]);
                        }
                        removed.push(k);
                        assert!(m.remove(&k).is_some());
                    }
                    Churn::Update(i) if !order.is_empty() => {
                        let k = order[i as usize % order.len()];
                        ver[k as usize] = ver[k as usize].wrapping_add(1);
                        m.insert(k, val(k, ver[k as usize]));
                    }
                    _ => {}
                }
            }
            testing::end_call();
            step += 1;
        }
        let mut sorted = seen.clone();
        sorted.sort();
        let before = sorted.len();
        sorted.dedup();
        if s.offset_mode || s.insert_only {
            assert_eq!(sorted.len(), before, "a key was returned twice: {seen:?}");
        }
        if gone {
            return; // the scan ended at CursorGone; completeness is not claimed
        }
        for k in &initial {
            if !removed.contains(k) && !seen.contains(k) {
                assert!(!s.insert_only, "insert-only churn skipped initial key {k}");
                assert!(moved.contains(k), "key {k} was skipped without being moved");
            }
        }
    });
}

/// The stated skip, pinned: offset paging over [0, 1, 2, 3] with pages of 2; between pages
/// key 0 is removed, which moves key 3 into slot 0, behind the offset; key 3 is never
/// returned.
#[test]
fn c6_pinned_offset_paging_skips_a_moved_key() {
    testing::use_mock();
    let mut m = flat();
    for k in 0..4 {
        m.insert(k, vec![k]);
    }
    testing::end_call();
    let p1: Vec<u8> = flat().iter().take(2).map(|(k, _)| k).collect();
    testing::end_call();
    let mut m = flat();
    m.remove(&0);
    testing::end_call();
    let p2: Vec<u8> = flat().iter().skip(2).take(2).map(|(k, _)| k).collect();
    testing::end_call();
    assert_eq!((p1, p2), (vec![0, 1], vec![2]), "key 3 skipped");
}

/// Cursor paging DUPLICATES, pinned (doc correction): [0..9) in pages of 8 returns 0..=7 with
/// cursor 7; two removals of slot 0 (keys 0, then 8) move key 7 into slot 0; the next page
/// after cursor 7 returns 1..=6 again.
#[test]
fn c6_pinned_cursor_paging_can_return_keys_twice() {
    testing::use_mock();
    let mut m = flat();
    for k in 0..9 {
        m.insert(k, vec![k]);
    }
    testing::end_call();
    let p1 = flat().keys_page(None, 8).unwrap();
    testing::end_call();
    assert_eq!((p1.keys.clone(), p1.next), ((0..8).collect::<Vec<u8>>(), Some(7)));
    let mut m = flat();
    m.remove(&0);
    m.remove(&8);
    testing::end_call();
    let p2 = flat().keys_page(Some(&7), 8).unwrap();
    testing::end_call();
    assert_eq!(p2.keys, (1..7).collect::<Vec<u8>>(), "keys 1..=6 returned a second time");
}
