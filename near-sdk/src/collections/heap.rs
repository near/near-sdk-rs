use borsh::{BorshDeserialize, BorshSerialize};

use crate::collections::{append, Vector, UnorderedMap};

/// Max-heap of elements, iterator returns natural ordering of elements (ascending).
#[derive(BorshSerialize, BorshDeserialize)]
pub struct Heap<T> {
    element_index_prefix: Vec<u8>,
    elements: Vector<T>,
    // keeps 1-based index in the max-heap of respective element
    indices: UnorderedMap<T, u64>,
}

impl<T> Heap<T>
    where
        T: BorshSerialize + BorshDeserialize + Ord
{

    pub fn new(id: Vec<u8>) -> Self {
        let element_index_prefix = append(&id, b'h');
        let elements_prefix = append(&id, b'k');
        let indices_prefix = append(&id, b'm');

        Self {
            element_index_prefix,
            elements: Vector::new(elements_prefix),
            indices: UnorderedMap::new(indices_prefix),
        }
    }

    pub fn len(&self) -> u64 {
        self.elements.len()
    }

    pub fn clear(&mut self) {
        self.elements.clear();
        self.indices.clear();
    }

    pub fn get_max(&self) -> Option<T> {
        self.at(0)
    }

    pub fn remove_max(&mut self) -> Option<T> {
        let max = self.get_max();
        if max.is_none() {
            return max;
        }
        let n = self.len();
        swap(&mut self.elements, 1, n, &mut self.indices);
        sink(&mut self.elements, 1, n - 1, &mut self.indices);
        self.elements.pop();
        max
    }

    pub fn insert(&mut self, value: &T) {
        if self.indices.get(value).is_some() {
            // value already exists in the heap, nothing to do
            return;
        }
        self.elements.push(value);
        let idx = self.elements.len();
        self.indices.insert(value, &idx);
        rise(&mut self.elements, idx, &mut self.indices);
    }

    pub fn remove(&mut self, value: &T) {
        let idx_opt = self.indices.get(&value);
        if idx_opt.is_none() {
            return
        }
        let idx = idx_opt.unwrap();
        let last = self.elements.len();
        swap(&mut self.elements, idx, last, &mut self.indices);
        self.elements.pop_raw();
        let n = self.len();
        sink(&mut self.elements, idx, n, &mut self.indices);
        self.indices.remove(&value);
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
fn swap<T: BorshSerialize + BorshDeserialize>(vec: &mut Vector<T>, i: u64, j: u64, indices: &mut UnorderedMap<T, u64>) {
    let i_opt = vec.get(i - 1);
    let j_opt = vec.get(j - 1);
    if i_opt.is_some() && j_opt.is_some() {
        vec.replace(i - 1, j_opt.as_ref().unwrap());
        vec.replace(j - 1, i_opt.as_ref().unwrap());
        indices.insert(i_opt.as_ref().unwrap(), &j);
        indices.insert(j_opt.as_ref().unwrap(), &i);
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
fn sink<T>(vec: &mut Vector<T>, mut idx: u64, n: u64, indices: &mut UnorderedMap<T, u64>)
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
        swap(vec, k, idx, indices);
        idx = k;
    }
}

// Takes 1-based index of element to rise (pop up)
fn rise<T>(vec: &mut Vector<T>, mut idx: u64, indices: &mut UnorderedMap<T, u64>)
    where
        T: Ord + BorshSerialize + BorshDeserialize
{
    while idx > 1 {
        let k = parent(idx);
        if !gt(vec, idx, k) {
            break;
        }
        swap(vec, idx, k, indices);
        idx = k;
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::test_env;

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
            swap(&mut vec, i, j, &mut UnorderedMap::new(vec![b't']));
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
            let mut map = UnorderedMap::new(vec![b't']);
            sink(&mut vec, idx, n, &mut map);
            let actual = vec.to_vec();
            vec.clear();
            map.clear();
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
            let mut map = UnorderedMap::new(vec![b't']);
            rise(&mut vec, idx, &mut map);
            let actual = vec.to_vec();
            vec.clear();
            map.clear();
            assert_eq!(actual, expected,
                       "rise({:?}, {}) expected {:?} but got {:?}", case, idx, expected, actual);
        }
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

            let actual = heap.into_iter().collect::<Vec<u8>>();
            assert_eq!(actual, sorted,
                       "Sorting {:?} failed: expected {:?} but got {:?}.", case, sorted, actual);
        }
    }

    #[test]
    fn test_iter_sorted_random() {
        test_env::setup();
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

        let cases = vec![10, 20, 30, 42]; // Error(GasLimitExceeded) for sizes >= 50
        for n in cases {
            let mut heap: Heap<u32> = Heap::new(vec![b't']);

            let items = random(n);
            for x in &items {
                heap.insert(x);
            }
            assert_eq!(heap.len(), n as u64);

            let mut sorted = items.clone();
            sorted.sort();

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
        assert!(heap.indices.get(&key).is_none());
        assert_eq!(heap.len(), 0);

        heap.insert(&key);
        assert_eq!(heap.len(), 1);
        assert_eq!(heap.indices.get(&key), Some(1));
        heap.clear();
    }

    #[test]
    fn test_insert_duplicate() {
        test_env::setup();

        let mut heap: Heap<u8> = Heap::new(vec![b't']);
        let key = 42u8;
        assert!(heap.indices.get(&key).is_none());
        assert_eq!(heap.len(), 0);

        heap.insert(&key);
        heap.insert(&key);
        assert_eq!(heap.len(), 1);
        assert_eq!(heap.indices.get(&key), Some(1));
        assert_eq!(heap.indices.len(), 1);
        assert_eq!(heap.elements.len(), 1);

        heap.clear();
    }

    #[test]
    fn test_remove() {
        test_env::setup();

        let mut heap: Heap<u8> = Heap::new(vec![b't']);
        let key = 42u8;
        assert!(heap.indices.get(&key).is_none());

        heap.insert(&key);
        heap.remove(&key);

        assert!(heap.indices.get(&key).is_none());
        assert_eq!(heap.len(), 0);

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

        let n = vec.len();
        for (i, _) in vec.iter().enumerate() {
            assert_eq!(heap.remove_max(),  Some((n - i) as u8));
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

}
