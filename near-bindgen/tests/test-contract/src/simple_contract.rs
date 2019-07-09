use near_bindgen::collections::Vec;
use near_bindgen::near_bindgen;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct SimpleContract {
    vec: Vec<u64>,
}

#[near_bindgen]
impl SimpleContract {
    pub fn pop(&mut self) {
        self.vec.pop();
    }

    pub fn to_vec(&self) -> std::vec::Vec<u64> {
        self.vec.to_vec()
    }
}

impl Default for SimpleContract {
    fn default() -> Self {
        let mut vec = Vec::default();
        vec.extend(0..5);
        Self { vec }
    }
}
