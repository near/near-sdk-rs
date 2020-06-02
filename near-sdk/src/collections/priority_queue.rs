use borsh::{BorshDeserialize, BorshSerialize};

use crate::collections::{append, Vector};

/// Priority Queue based on Max-Heap.
///
/// Iterator consumes the queue and returns all elements in descending order.
///
/// Runtime complexity (worst case):
/// - `offer`:  O(log(N))
/// - `poll`:   O(log(N))
/// - `peek`:   O(1)
///
#[derive(BorshSerialize, BorshDeserialize)]
pub struct PriorityQueue<T> {
    elements: Vector<T>,
}

impl<T> PriorityQueue<T>
    where
        T: BorshSerialize + BorshDeserialize + Ord
{

    pub fn new(id: Vec<u8>) -> Self {
        Self {
            elements: Vector::new(append(&id, b'e')),
        }
    }

    pub fn len(&self) -> u64 {
        self.elements.len()
    }

    pub fn clear(&mut self) {
        self.elements.clear();
    }

    /// Get current head of the queue and leave it in the queue
    pub fn peek(&self) -> Option<T> {
        self.elements.get(0)
    }

    /// Get current head of the queue and remove it from the queue
    pub fn poll(&mut self) -> Option<T> {
        match self.peek() {
            Some(max) => {
                let n = self.len();
                swap(&mut self.elements, 1, n);
                sink(&mut self.elements, 1, n - 1);
                self.elements.pop();
                Some(max)
            },
            None => None
        }
    }

    /// Insert element into the queue
    pub fn offer(&mut self, value: &T) {
        self.elements.push(value);
        let idx = self.elements.len();
        rise(&mut self.elements, idx);
    }

    pub fn into_iter(mut self) -> impl Iterator<Item = T> {
        (0..self.len()).map(move |_| self.poll().unwrap())
    }
}

fn zip<T, U>(lhs: Option<T>, rhs: Option<U>) -> Option<(T, U)> {
    if lhs.is_none() || rhs.is_none() {
        None
    } else {
        Some((lhs.unwrap(), rhs.unwrap()))
    }
}

// Takes 1-based indices of elements to swap
fn gt<T: Ord + BorshSerialize + BorshDeserialize>(vec: &Vector<T>, i: u64, j: u64) -> bool {
    (i != j) && (i > 0 && j > 0) && zip(vec.get(i - 1), vec.get(j - 1))
        .map(|(lhs, rhs)| lhs.gt(&rhs))
        .unwrap_or_default()
}

// Takes 1-based indices of elements to swap
fn swap<T: BorshSerialize + BorshDeserialize>(vec: &mut Vector<T>, i: u64, j: u64) {
    let i_opt = vec.get(i - 1);
    let j_opt = vec.get(j - 1);
    if i_opt.is_some() && j_opt.is_some() {
        vec.replace(i - 1, j_opt.as_ref().unwrap());
        vec.replace(j - 1, i_opt.as_ref().unwrap());
    }
}

// 1-based index calculation
fn parent(i: u64) -> u64 {
    i / 2
}

// 1-based index calculation
fn child(i: u64) -> u64 {
    i * 2
}

// Takes 1-based index of element to sink
fn sink<T>(vec: &mut Vector<T>, mut idx: u64, n: u64)
    where
        T: Ord + BorshSerialize + BorshDeserialize
{
    while child(idx) <= n {
        let mut k = child(idx);
        if k < n && gt(vec, k + 1, k) {
            k += 1;
        }
        if !gt(vec, k, idx) {
            break;
        }
        swap(vec, k, idx);
        idx = k;
    }
}

// Takes 1-based index of element to rise (pop up)
fn rise<T>(vec: &mut Vector<T>, mut idx: u64)
    where
        T: Ord + BorshSerialize + BorshDeserialize
{
    while idx > 1 {
        let k = parent(idx);
        if !gt(vec, idx, k) {
            break;
        }
        swap(vec, idx, k);
        idx = k;
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::test_env;
    use quickcheck::QuickCheck;

    #[test]
    fn test_empty() {
        let id = vec![b'x'];
        let queue: PriorityQueue<u8> = PriorityQueue::new(id);
        assert_eq!(0, queue.len());
    }

    #[test]
    fn test_zip() {
        let (some, none) = (Some(1), None as Option<u32>);
        let (full, empty) = (Some((1, 1)), None as Option<(u32, u32)>);
        assert_eq!(zip(none, none), empty);
        assert_eq!(zip(some, none), empty);
        assert_eq!(zip(none, some), empty);
        assert_eq!(zip(some, some), full);
    }

    #[test]
    fn test_less() {
        test_env::setup();
        let mut vec: Vector<i32> = Vector::new(vec![b'x']);
        vec.push(&2);
        vec.push(&1);
        vec.push(&1);
        assert!(gt(&vec, 1, 2));
        assert!(!gt(&vec, 2, 1));
        assert!(!gt(&vec, 1, 1));
        assert!(!gt(&vec, 2, 2));
        assert!(!gt(&vec, 2, 4));
        assert!(!gt(&vec, 4, 1));
        assert!(!gt(&vec, 2, 3));
        assert!(!gt(&vec, 3, 2));
        vec.clear();
    }

    #[test]
    fn test_swap() {
        test_env::setup();

        let cases: Vec<((u64, u64), Vec<u8>)> = vec![
            ((1, 2), vec![2u8, 1u8]),
            ((1, 3), vec![1u8, 2u8]),
            ((3, 2), vec![1u8, 2u8]),
        ];

        for ((i, j), expected) in cases {
            let mut vec: Vector<u8> = Vector::new(vec![b'x']);
            vec.push(&1);
            vec.push(&2);
            swap(&mut vec, i, j);
            let actual = vec.to_vec();
            vec.clear();
            assert_eq!(actual, expected);
        }
    }

    #[test]
    fn test_sink() {
        test_env::setup();

        let cases: Vec<(Vec<u8>, u64, Vec<u8>)> = vec![
            (vec![1, 2], 1, vec![2, 1]),
            (vec![2, 1], 1, vec![2, 1]),
            (vec![1, 2, 3], 1, vec![3, 2, 1]),
            (vec![1, 2, 3], 4, vec![1, 2, 3]),
            (vec![1, 2, 3], 0, vec![1, 2, 3]),
            (vec![2, 1, 3], 1, vec![3, 1, 2]),
            (vec![2, 1, 1], 1, vec![2, 1, 1]),
            (vec![2, 1, 3], 1, vec![3, 1, 2]),
            (vec![2, 3, 1], 1, vec![3, 2, 1]),
            (vec![1, 3, 2, 4], 1, vec![3, 4, 2, 1]),
            (vec![1, 4, 3, 2, 5], 1, vec![4, 5, 3, 2, 1]),
            (vec![1, 2, 5, 4, 7], 2, vec![1, 7, 5, 4, 2]),
            (vec![1, 2, 3, 4, 5, 7], 3, vec![1, 2, 7, 4, 5, 3]),
            (vec![7, 1, 4, 2, 5, 3], 2, vec![7, 5, 4, 2, 1, 3]),
        ];

        for (case, idx, expected) in cases {
            let mut vec: Vector<u8> = Vector::new(vec![b'x']);
            for x in &case {
                vec.push(x);
            }
            let n = vec.len();
            sink(&mut vec, idx, n);
            let actual = vec.to_vec();
            vec.clear();
            assert_eq!(actual, expected,
                       "sink({:?}, {}) expected {:?} but got {:?}", case, idx, expected, actual);
        }
    }

    #[test]
    fn test_rise() {
        test_env::setup();

        let cases: Vec<(Vec<u8>, u64, Vec<u8>)> = vec![
            (vec![], 1, vec![]),
            (vec![1, 2], 2, vec![2, 1]),
            (vec![2, 1], 2, vec![2, 1]),
            (vec![1, 2, 3], 3, vec![3, 2, 1]),
            (vec![1, 2, 3, 4, 5], 5, vec![5, 1, 3, 4, 2]),
        ];

        for (case, idx, expected) in cases {
            let mut vec: Vector<u8> = Vector::new(vec![b'x']);
            for x in &case {
                vec.push(x);
            }
            rise(&mut vec, idx);
            let actual = vec.to_vec();
            vec.clear();
            assert_eq!(actual, expected,
                       "rise({:?}, {}) expected {:?} but got {:?}", case, idx, expected, actual);
        }
    }

    #[test]
    fn test_iter_empty() {
        let queue: PriorityQueue<u8> = PriorityQueue::new(vec![b't']);
        assert!(queue.into_iter().collect::<Vec<u8>>().is_empty());
    }

    #[test]
    fn test_iter_sorted() {
        test_env::setup();

        let cases: Vec<Vec<u8>> = vec![
            vec![1, 2],
            vec![2, 1],
            vec![1, 2, 3],
            vec![1, 3, 2],
            vec![2, 1, 3],
            vec![2, 3, 1],
            vec![3, 2, 1],
            vec![3, 1, 2],
            vec![1, 2, 3, 4],
            vec![4, 3, 2, 1],
            vec![4, 2, 3, 1],
            vec![3, 1, 2, 4],
            (0..5).collect(),
            (0..5).rev().collect(),
            (0..10).collect(),
            (0..10).rev().collect(),
        ];

        for case in cases {
            let mut queue: PriorityQueue<u8> = PriorityQueue::new(vec![b't']);
            for x in &case {
                queue.offer(&x);
            }
            assert_eq!(queue.len(), case.len() as u64);

            let mut sorted = case.clone();
            sorted.sort();
            sorted.reverse();

            let actual = queue.into_iter().collect::<Vec<u8>>();
            assert_eq!(actual, sorted,
                       "Sorting {:?} failed: expected {:?} but got {:?}.", case, sorted, actual);
        }
    }

    #[test]
    fn test_iter_sorted_random() {
        test_env::setup_free();
        use rand::prelude::*;

        fn random(n: u32) -> Vec<u32> {
            let mut vec = Vec::with_capacity(n as usize);
            for x in 0..n {
                vec.push(x);
            }
            let mut rng = rand::thread_rng();
            vec.shuffle(&mut rng);
            vec
        }

        let cases = vec![10, 20, 30, 40, 50, 60, 70];
        for n in cases {
            let mut queue: PriorityQueue<u32> = PriorityQueue::new(vec![b't']);

            let items = random(n);
            for x in &items {
                queue.offer(x);
            }
            assert_eq!(queue.len(), n as u64);

            let mut sorted = items.clone();
            sorted.sort();
            sorted.reverse();

            let actual = queue.into_iter().collect::<Vec<u32>>();
            assert_eq!(actual, sorted,
                       "Sorting {:?} failed: expected {:?} but got {:?}.", items, sorted, actual);
        }
    }

    #[test]
    fn test_offer() {
        test_env::setup();

        let mut queue: PriorityQueue<u8> = PriorityQueue::new(vec![b't']);
        let key = 42u8;
        assert_eq!(queue.len(), 0);

        queue.offer(&key);
        assert_eq!(queue.len(), 1);
        queue.clear();
    }

    #[test]
    fn test_offer_duplicate() {
        test_env::setup();

        let mut queue: PriorityQueue<u8> = PriorityQueue::new(vec![b't']);
        let key = 42u8;
        assert_eq!(queue.len(), 0);

        let k = 3;
        for _ in 0..k {
            queue.offer(&key);
        }
        assert_eq!(queue.len(), k);

        queue.clear();
    }

    #[test]
    fn test_peek() {
        test_env::setup();
        let mut queue: PriorityQueue<u8> = PriorityQueue::new(vec![b't']);

        for x in vec![1u8, 2u8, 3u8, 4u8, 5u8] {
            queue.offer(&x);
        }

        assert_eq!(queue.peek(), Some(5u8));
        assert_eq!(queue.len(), 5);

        queue.clear();
    }

    #[test]
    fn test_poll() {
        test_env::setup();
        let mut queue: PriorityQueue<u8> = PriorityQueue::new(vec![b't']);

        let vec = vec![1u8, 2u8, 3u8, 4u8, 5u8];
        for x in vec.iter() {
            queue.offer(&x);
        }
        assert_eq!(queue.len(), vec.len() as u64);

        let n = vec.len();
        for (i, x) in vec.iter().rev().enumerate() {
            assert_eq!(queue.poll(), Some(*x));
            assert_eq!(queue.len() as usize, n - 1 - i);
        }

        assert_eq!(queue.len(), 0);

        queue.clear();
    }

    #[test]
    fn test_poll_duplicates() {
        test_env::setup();
        let mut queue: PriorityQueue<u8> = PriorityQueue::new(vec![b't']);

        let vec: Vec<u8> = vec![1, 1, 1, 2, 2, 2, 3, 3, 3, 4, 4, 4, 5, 5, 5];
        for x in vec.iter() {
            queue.offer(&x);
        }
        assert_eq!(queue.len(), vec.len() as u64);

        let n = vec.len();
        for (i, x) in vec.iter().rev().enumerate() {
            assert_eq!(queue.poll(), Some(*x));
            assert_eq!(queue.len() as usize, n - 1 - i);
        }

        assert_eq!(queue.len(), 0);

        queue.clear();
    }

    #[test]
    fn test_peek_empty() {
        test_env::setup();
        let mut queue: PriorityQueue<u8> = PriorityQueue::new(vec![b't']);

        assert_eq!(queue.peek(), None);

        queue.clear();
    }

    #[test]
    fn test_poll_empty() {
        test_env::setup();
        let mut queue: PriorityQueue<u8> = PriorityQueue::new(vec![b't']);

        assert_eq!(queue.poll(), None);

        queue.clear();
    }

    #[test]
    fn prop_max_heap() {
        test_env::setup_free();

        fn prop(offer: Vec<u32>) -> bool {
            let mut queue = PriorityQueue::new(vec![b't']);
            for x in offer.iter() {
                queue.offer(x);
            }

            let n = queue.len();
            (0..n).all(|i| {
                let m = queue.elements.get(0).unwrap();

                let c1 = child(i + 1) - 1; // 1-based
                let c2 = c1 + 1;

                queue.elements.get(c1).map(|x| x.le(&m)).unwrap_or(true) &&
                    queue.elements.get(c2).map(|x| x.le(&m)).unwrap_or(true)
            })
        }

        QuickCheck::new()
            .tests(300)
            .quickcheck(prop as fn(Vec<u32>) -> bool);
    }

    #[test]
    fn prop_queue_iter() {
        test_env::setup_free();

        fn prop(mut offer: Vec<u32>) -> bool {
            let mut queue = PriorityQueue::new(vec![b't']);
            for x in offer.iter() {
                queue.offer(x);
            }

            offer.sort();
            offer.reverse();

            queue.into_iter().collect::<Vec<u32>>() == offer
        }

        QuickCheck::new()
            .tests(300)
            .quickcheck(prop as fn(Vec<u32>) -> bool);
    }
}
