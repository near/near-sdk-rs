use near_vm_logic::MemoryLike;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    // Returns whether the memory interval is completely inside the smart contract memory.
    pub fn fits_memory(offset: u64, len: u64) -> bool;

    // Reads the content of the given memory interval.
    //
    // # Panics
    //
    // If memory interval is outside the smart contract memory.
    pub fn read_memory(offset: u64, buffer: &mut [u8]);

    // Reads a single byte from the memory.
    //
    // # Panics
    //
    // If pointer is outside the smart contract memory.
    pub fn read_memory_u8(offset: u64) -> u8;

    // Writes the buffer into the smart contract memory.
    //
    // # Panics
    //
    // If `offset + buffer.len()` is outside the smart contract memory.
    pub fn write_memory(offset: u64, buffer: &[u8]);

    pub fn alert(s: &str);
}

pub struct MockedMemory {}

impl MemoryLike for MockedMemory {
    fn fits_memory(&self, _offset: u64, _len: u64) -> bool {
        fits_memory(_offset, _len)
    }

    fn read_memory(&self, offset: u64, buffer: &mut [u8]) {
        read_memory(offset, buffer)
    }

    fn read_memory_u8(&self, offset: u64) -> u8 {
        read_memory_u8(offset)
    }

    fn write_memory(&mut self, offset: u64, buffer: &[u8]) {
        write_memory(offset, buffer)
    }
}
