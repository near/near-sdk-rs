use borsh::{BorshDeserialize, BorshSerialize};

use crate::env;
use crate::collections::append;
use crate::collections::{Vector, UnorderedMap, ERR_ELEMENT_SERIALIZATION};

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
        let element_index_prefix = append(&id, b'i');
        let elements_prefix = append(&id, b'e');
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

    pub fn at(&self, idx: u64) -> Option<T> {
        self.elements.get(idx)
    }

    pub fn insert(&mut self, value: &T) {
        self.elements.push_raw(serialize(value).as_slice());
        let idx = self.elements.len() - 1;
        self.indices.insert(value, &idx);
        rise(&mut self.elements, idx);
    }

    pub fn remove(&mut self, value: &T) {
        let idx_opt = self.indices.get(&value);
        if idx_opt.is_none() {
            return
        }
        let idx = idx_opt.unwrap();
        let last = self.elements.len() - 1;
        swap(&mut self.elements, idx, last);
        self.elements.pop_raw();
        sink(&mut self.elements, idx);
    }

    pub fn iter<'a>(&'a self) -> impl Iterator<Item = T> + 'a {
        // TODO sort values
        self.elements.iter()
    }
}

fn serialize<T: BorshSerialize>(value: &T) -> Vec<u8> {
    value.try_to_vec().unwrap_or_else(|_| env::panic(ERR_ELEMENT_SERIALIZATION))
}

fn zip<T, U>(lhs: Option<T>, rhs: Option<U>) -> Option<(T, U)> {
    if lhs.is_none() || rhs.is_none() {
        None
    } else {
        Some((lhs.unwrap(), rhs.unwrap()))
    }
}

fn less<T: Ord + BorshSerialize + BorshDeserialize>(vec: &Vector<T>, i: u64, j: u64) -> bool {
    (i != j) && zip(vec.get(i), vec.get(j))
        .map(|(lhs, rhs)| lhs.lt(&rhs))
        .unwrap_or_default()
}

fn swap<T: BorshSerialize + BorshDeserialize>(vec: &mut Vector<T>, i: u64, j: u64) {
    let i_opt = vec.get_raw(i);
    let j_opt = vec.get_raw(j);
    if i_opt.is_some() && j_opt.is_some() {
        vec.replace_raw(i, j_opt.as_ref().unwrap().as_slice());
        vec.replace_raw(j, i_opt.as_ref().unwrap().as_slice());
        // TODO update `indices` here
    }
}

fn sink<T>(vec: &mut Vector<T>, mut idx: u64)
    where
        T: Ord + BorshSerialize + BorshDeserialize
{
    while idx * 2 < vec.len() {
        let mut k = idx * 2;
        if k < (vec.len() - 1) && less(vec, k, k + 1) {
            k += 1;
        }
        if less(vec, idx, k) {
            swap(vec, idx, k);
        }
        idx = k;
    }
}

fn rise<T>(vec: &mut Vector<T>, mut idx: u64)
    where
        T: Ord + BorshSerialize + BorshDeserialize
{
    while idx / 2 > 0 {
        let k = idx / 2;
        if less(vec, idx, k) {
            swap(vec, idx, k);
        }
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
        vec.push(&1);
        vec.push(&2);
        assert!(less(&vec, 0, 1));
        assert!(!less(&vec, 1, 0));
        assert!(!less(&vec, 0, 0));
        assert!(!less(&vec, 1, 1));
        assert!(!less(&vec, 1, 3));
        assert!(!less(&vec, 3, 0));
    }

    #[test]
    fn test_swap() {
        test_env::setup();

        let cases: Vec<((u64, u64), Vec<u8>)> = vec![
            ((0, 1), vec![2u8, 1u8]),
            ((0, 2), vec![1u8, 2u8]),
            ((2, 1), vec![1u8, 2u8]),
        ];

        for ((i, j), expected) in cases {
            let mut vec: Vector<u8> = Vector::new(vec![b'x']);
            vec.push(&1);
            vec.push(&2);
            swap(&mut vec, i, j);
            let actual = vec.to_vec();
            assert_eq!(actual, expected);
        }
    }

    // TODO test_sink
    // TODO test_rise
}
