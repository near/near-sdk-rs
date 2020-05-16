use borsh::{BorshDeserialize, BorshSerialize};

use crate::env;
use crate::collections::{Vector, ERR_ELEMENT_SERIALIZATION};

#[derive(BorshSerialize, BorshDeserialize)]
pub struct Heap<T> {
    element_index_prefix: Vec<u8>,
    elements: Vector<T>,
}

impl<T> Heap<T>
    where
        T: BorshSerialize + BorshDeserialize + Ord
{

    pub fn new(id: Vec<u8>) -> Self {
        let mut element_index_prefix = Vec::with_capacity(id.len() + 1);
        element_index_prefix.extend(&id);
        element_index_prefix.push(b'i');

        let mut elements_prefix = Vec::with_capacity(id.len() + 1);
        elements_prefix.extend(&id);
        elements_prefix.push(b'e');

        Self { element_index_prefix, elements: Vector::new(elements_prefix) }
    }

    pub fn len(&self) -> u64 {
        self.elements.len()
    }

    pub fn at(&self, idx: u64) -> Option<T> {
        self.elements.get(idx)
    }

    pub fn insert(&mut self, value: &T) -> Vec<(u64, u64)> {
        self.elements.push_raw(serialize(value).as_slice());
        let idx = self.elements.len() - 1;
        let mut swaps = Vec::with_capacity(log2_ceil(self.elements.len()));
        rise(&mut self.elements, idx, &mut swaps);
        swaps
    }

    pub fn remove(&mut self, idx: u64) -> Vec<(u64, u64)> {
        let last = self.elements.len() - 1;
        swap(&mut self.elements, idx, last);
        self.elements.pop_raw();
        let mut swaps = Vec::with_capacity(log2_ceil(self.elements.len()));
        sink(&mut self.elements, idx, &mut swaps);
        swaps
    }

    pub fn iter<'a>(&'a self) -> impl Iterator<Item = T> + 'a {
        // TODO sort values
        self.elements.iter()
    }
}

fn serialize<T: BorshSerialize>(value: &T) -> Vec<u8> {
    value.try_to_vec().unwrap_or_else(|_| env::panic(ERR_ELEMENT_SERIALIZATION))
}

fn log2_ceil(x: u64) -> usize {
    (x as f64).log2().ceil() as usize
}

fn zip<T, U>(lhs: Option<T>, rhs: Option<U>) -> Option<(T, U)> {
    if lhs.is_none() || rhs.is_none() {
        None
    } else {
        Some((lhs.unwrap(), rhs.unwrap()))
    }
}

fn less<T: Ord + BorshSerialize + BorshDeserialize>(vec: &Vector<T>, i: u64, j: u64) -> bool {
    zip(vec.get(i), vec.get(j))
        .map(|(lhs, rhs)| lhs.lt(&rhs))
        .unwrap_or_default()
}

fn swap<T: BorshSerialize + BorshDeserialize>(vec: &mut Vector<T>, i: u64, j: u64) {
    let i_opt = vec.get_raw(i);
    let j_opt = vec.get_raw(j);
    if i_opt.is_some() && j_opt.is_some() {
        vec.replace_raw(i, j_opt.unwrap().as_slice());
        vec.replace_raw(j, i_opt.unwrap().as_slice());
    }
}

fn sink<T>(vec: &mut Vector<T>, mut idx: u64, swaps: &mut Vec<(u64, u64)>)
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
            swaps.push((idx, k));
        }
        idx = k;
    }
}

fn rise<T>(vec: &mut Vector<T>, mut idx: u64, swaps: &mut Vec<(u64, u64)>)
    where
        T: Ord + BorshSerialize + BorshDeserialize
{
    while idx / 2 > 0 {
        let k = idx / 2;
        if less(vec, idx, k) {
            swap(vec, idx, k);
            swaps.push((idx, k));
        }
        idx = k;
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty() {
        let id = vec![b'x'];
        let heap: Heap<u8> = Heap::new(id);
        assert_eq!(0, heap.len());
    }
}
