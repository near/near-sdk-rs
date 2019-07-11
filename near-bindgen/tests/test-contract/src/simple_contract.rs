use near_bindgen::collections::Vec;
use near_bindgen::near_bindgen;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct SimpleContract {
    vec: Vec<u64>,
}

#[near_bindgen]
impl SimpleContract {
    pub fn vec_clear(&mut self) {
        self.vec.clear();
    }

    pub fn vec_len(&self) -> usize {
        self.vec.len()
    }

    pub fn vec_remove(&mut self, index: usize) -> u64 {
        self.vec.remove(index)
    }

    pub fn vec_insert(&mut self, index: usize, element: u64) {
        self.vec.insert(index, element);
    }

    pub fn vec_get(&self, index: usize) -> Option<u64> {
        self.vec.get(index)
    }

    pub fn vec_pop(&mut self) {
        self.vec.pop();
    }

    pub fn vec_push(&mut self, value: u64) {
        self.vec.push(value);
    }

    pub fn vec_first(&self) -> Option<u64> {
        self.vec.first()
    }

    pub fn vec_last(&self) -> Option<u64> {
        self.vec.last()
    }

    pub fn vec_to_vec(&self) -> std::vec::Vec<u64> {
        self.vec.to_vec()
    }

    pub fn vec_drain(&mut self, start: usize, end: usize) -> std::vec::Vec<u64> {
        self.vec.drain(start..end).collect()
    }
}

impl Default for SimpleContract {
    fn default() -> Self {
        let mut vec = Vec::default();
        vec.extend(0..5);
        Self { vec }
    }
}
