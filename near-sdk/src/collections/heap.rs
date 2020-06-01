use borsh::{BorshDeserialize, BorshSerialize};

use crate::collections::{append, Vector};

/// Max-heap
///
/// Iterator consumes the heap and returns all elements in descending order.
///
/// Runtime complexity (worst case):
/// - `insert`:     O(log(N))
/// - `remove_max`: O(log(N))
/// - `get_max`:    O(1)
/// - iterate:      O(Nlog(N))
///
#[derive(BorshSerialize, BorshDeserialize)]
pub struct Heap<T> {
    elements: Vector<T>,
}

impl<T> Heap<T>
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

    pub fn get_max(&self) -> Option<T> {
        self.at(0)
    }

    pub fn remove_max(&mut self) -> Option<T> {
        match self.get_max() {
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

    pub fn insert(&mut self, value: &T) {
        self.elements.push(value);
        let idx = self.elements.len();
        rise(&mut self.elements, idx);
    }

    pub fn into_iter(self) -> impl Iterator<Item = T> {
        HeapIterator::of(self)
    }

    fn at(&self, idx: u64) -> Option<T> {
        self.elements.get(idx)
    }
}

pub struct HeapIterator<T> {
    heap: Heap<T>
}

impl<T> HeapIterator<T> {
    fn of(heap: Heap<T>) -> Self {
        Self {
            heap
        }
    }
}

impl<T> Iterator for HeapIterator<T>
    where
        T: BorshSerialize + BorshDeserialize + Ord
{
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        self.heap.remove_max()
    }
}

impl<T> IntoIterator for Heap<T>
    where
        T: BorshSerialize + BorshDeserialize + Ord
{
    type Item = T;
    type IntoIter = HeapIterator<T>;

    fn into_iter(self) -> Self::IntoIter {
        HeapIterator {
            heap: self,
        }
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
        let heap: Heap<u8> = Heap::new(id);
        assert_eq!(0, heap.len());
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
        let heap: Heap<u8> = Heap::new(vec![b't']);
        assert!(heap.into_iter().collect::<Vec<u8>>().is_empty());
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
            let mut heap: Heap<u8> = Heap::new(vec![b't']);
            for x in &case {
                heap.insert(&x);
            }
            assert_eq!(heap.len(), case.len() as u64);

            let mut sorted = case.clone();
            sorted.sort();
            sorted.reverse();

            let actual = heap.into_iter().collect::<Vec<u8>>();
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
            let mut heap: Heap<u32> = Heap::new(vec![b't']);

            let items = random(n);
            for x in &items {
                heap.insert(x);
            }
            assert_eq!(heap.len(), n as u64);

            let mut sorted = items.clone();
            sorted.sort();
            sorted.reverse();

            let actual = heap.into_iter().collect::<Vec<u32>>();
            assert_eq!(actual, sorted,
                       "Sorting {:?} failed: expected {:?} but got {:?}.", items, sorted, actual);
        }
    }

    #[test]
    fn test_insert() {
        test_env::setup();

        let mut heap: Heap<u8> = Heap::new(vec![b't']);
        let key = 42u8;
        assert_eq!(heap.len(), 0);

        heap.insert(&key);
        assert_eq!(heap.len(), 1);
        heap.clear();
    }

    #[test]
    fn test_insert_duplicate() {
        test_env::setup();

        let mut heap: Heap<u8> = Heap::new(vec![b't']);
        let key = 42u8;
        assert_eq!(heap.len(), 0);

        let k = 3;
        for _ in 0..k {
            heap.insert(&key);
        }
        assert_eq!(heap.len(), k);

        heap.clear();
    }

    #[test]
    fn test_get_max() {
        test_env::setup();
        let mut heap: Heap<u8> = Heap::new(vec![b't']);

        for x in vec![1u8, 2u8, 3u8, 4u8, 5u8] {
            heap.insert(&x);
        }

        assert_eq!(heap.get_max(), Some(5u8));
        assert_eq!(heap.len(), 5);

        heap.clear();
    }

    #[test]
    fn test_remove_max() {
        test_env::setup();
        let mut heap: Heap<u8> = Heap::new(vec![b't']);

        let vec = vec![1u8, 2u8, 3u8, 4u8, 5u8];
        for x in vec.iter() {
            heap.insert(&x);
        }
        assert_eq!(heap.len(), vec.len() as u64);

        let n = vec.len();
        for (i, x) in vec.iter().rev().enumerate() {
            assert_eq!(heap.remove_max(),  Some(*x));
            assert_eq!(heap.len() as usize, n - 1 - i);
        }

        assert_eq!(heap.len(), 0);

        heap.clear();
    }

    #[test]
    fn test_remove_max_duplicates() {
        test_env::setup();
        let mut heap: Heap<u8> = Heap::new(vec![b't']);

        let vec: Vec<u8> = vec![1, 1, 1, 2, 2, 2, 3, 3, 3, 4, 4, 4, 5, 5, 5];
        for x in vec.iter() {
            heap.insert(&x);
        }
        assert_eq!(heap.len(), vec.len() as u64);

        let n = vec.len();
        for (i, x) in vec.iter().rev().enumerate() {
            assert_eq!(heap.remove_max(),  Some(*x));
            assert_eq!(heap.len() as usize, n - 1 - i);
        }

        assert_eq!(heap.len(), 0);

        heap.clear();
    }

    #[test]
    fn test_remote_max_empty() {
        test_env::setup();
        let mut heap: Heap<u8> = Heap::new(vec![b't']);

        assert_eq!(heap.remove_max(), None);

        heap.clear();
    }

    #[test]
    fn prop_max_heap() {
        test_env::setup_free();

        fn prop(insert: Vec<u32>) -> bool {
            let mut heap = Heap::new(vec![b't']);
            for x in insert.iter() {
                heap.insert(x);
            }

            let n = heap.len();
            (0..n).all(|i| {
                let m = heap.elements.get(0).unwrap();

                let c1 = child(i + 1) - 1; // 1-based
                let c2 = c1 + 1;

                heap.elements.get(c1).map(|x| x.le(&m)).unwrap_or(true) &&
                heap.elements.get(c2).map(|x| x.le(&m)).unwrap_or(true)
            })
        }

        QuickCheck::new()
            .tests(300)
            .quickcheck(prop as fn(Vec<u32>) -> bool);
    }

    #[test]
    fn prop_max_heap_iter() {
        test_env::setup_free();

        fn prop(mut insert: Vec<u32>) -> bool {
            let mut heap = Heap::new(vec![b't']);
            for x in insert.iter() {
                heap.insert(x);
            }

            insert.sort();
            insert.reverse();

            heap.into_iter().collect::<Vec<u32>>() == insert
        }

        QuickCheck::new()
            .tests(300)
            .quickcheck(prop as fn(Vec<u32>) -> bool);
    }
}
