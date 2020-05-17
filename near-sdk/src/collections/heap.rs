use borsh::{BorshDeserialize, BorshSerialize};

use crate::collections::{append, Vector, UnorderedMap};

#[derive(BorshSerialize, BorshDeserialize)]
pub struct Heap<T> {
    element_index_prefix: Vec<u8>,
    elements: Vector<T>,
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

    pub fn at(&self, idx: u64) -> Option<T> {
        self.elements.get(idx)
    }

    pub fn lookup(&self, value: &T) -> Option<u64> {
        self.indices.get(value)
    }

    pub fn insert(&mut self, value: &T) {
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

    pub fn iter<'a>(&'a mut self) -> impl Iterator<Item = T> + 'a {
        sort(&mut self.elements, &mut self.indices);
        self.elements.iter()
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
fn less<T: Ord + BorshSerialize + BorshDeserialize>(vec: &Vector<T>, i: u64, j: u64) -> bool {
    (i != j) && zip(vec.get(i - 1), vec.get(j - 1))
        .map(|(lhs, rhs)| lhs.lt(&rhs))
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
        if k < n && less(vec, k + 1, k) {
            k += 1;
        }
        if !less(vec, k, idx) {
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
        if !less(vec, idx, k) {
            break;
        }
        swap(vec, idx, k, indices);
        idx = k;
    }
}

fn sort<T>(vec: &mut Vector<T>, indices: &mut UnorderedMap<T, u64>)
    where
        T: Ord + BorshSerialize + BorshDeserialize
{
    // Sort vector in-place in O(Nlog(N))
    let n = vec.len();
    let mut k = n;
    while k > 1 {
        swap(vec, 1, k, indices);
        k -= 1;
        sink(vec, 1, k, indices);
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
        vec.push(&1);
        vec.push(&2);
        vec.push(&2);
        assert!(less(&vec, 1, 2));
        assert!(!less(&vec, 2, 1));
        assert!(!less(&vec, 1, 1));
        assert!(!less(&vec, 2, 2));
        assert!(!less(&vec, 2, 4));
        assert!(!less(&vec, 4, 1));
        assert!(!less(&vec, 2, 3));
        assert!(!less(&vec, 3, 2));
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
            (vec![1, 2], 1, vec![1, 2]),
            (vec![2, 1], 1, vec![1, 2]),
            (vec![1, 2, 3], 1, vec![1, 2, 3]),
            (vec![1, 2, 3], 4, vec![1, 2, 3]),
            (vec![2, 1, 3], 1, vec![1, 2, 3]),
            (vec![2, 1, 1], 1, vec![1, 2, 1]),
            (vec![3, 1, 2], 1, vec![1, 3, 2]),
            (vec![2, 3, 1], 1, vec![1, 3, 2]),
            (vec![3, 1, 2, 4], 1, vec![1, 3, 2, 4]),
            (vec![1, 2, 3, 4, 5], 1, vec![1, 2, 3, 4, 5]),
            (vec![7, 2, 3, 4, 5], 1, vec![2, 4, 3, 7, 5]),
            (vec![1, 2, 7, 4, 5], 3, vec![1, 2, 7, 4, 5]),
            (vec![1, 2, 7, 4, 5, 3], 3, vec![1, 2, 3, 4, 5, 7]),
            (vec![1, 7, 2, 4, 5, 3], 2, vec![1, 4, 2, 7, 5, 3]),
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
            (vec![2, 3, 1], 3, vec![1, 3, 2]),
            (vec![5, 4, 3, 2, 1], 5, vec![1, 5, 3, 2, 4]),
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
    fn test_sort() {
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

            // TODO inverse the order of sorted vector
            let rev = sorted.into_iter().rev().collect::<Vec<u8>>();

            let actual = heap.iter().collect::<Vec<u8>>();
            heap.clear();
            assert_eq!(actual, rev,
                       "Sorting {:?} failed: expected {:?} but got {:?}.", case, rev, actual);
        }
    }

    // TODO test_insert
    // TODO test_remove

}
