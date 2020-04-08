use near_vm_logic::MemoryLike;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    // #[wasm_bindgen(js_name = Memory)]
    #[derive(Clone)]
    pub type NearMemory;

    // Returns whether the memory interval is completely inside the smart contract memory.
    #[wasm_bindgen(structural, method)]
    pub fn fits_memory(this: &NearMemory, offset: u64, len: u64) -> bool;

    // Reads the content of the given memory interval.
    //
    // # Panics
    //
    // If memory interval is outside the smart contract memory.
    #[wasm_bindgen(structural, method)]
    pub fn read_memory(this: &NearMemory, offset: u64, buffer: &mut [u8]);

    // Reads a single byte from the memory.
    //
    // # Panics
    //
    // If pointer is outside the smart contract memory.
    #[wasm_bindgen(structural, method)]
    pub fn read_memory_u8(this: &NearMemory, offset: u64) -> u8;

    // Writes the buffer into the smart contract memory.
    //
    // # Panics
    //
    // If `offset + buffer.len()` is outside the smart contract memory.
    #[wasm_bindgen(structural, method)]
    pub fn write_memory(this: &NearMemory, offset: u64, buffer: &[u8]);

    pub fn alert(s: &str);
}

#[derive(Clone)]
pub struct MockedMemory {
    pub mem: NearMemory
}

impl MemoryLike for MockedMemory {

    fn fits_memory(&self, _offset: u64, _len: u64) -> bool {
        self.mem.fits_memory(_offset, _len)
    }

    fn read_memory(&self, offset: u64, buffer: &mut [u8]) {
        self.mem.read_memory(offset, buffer)
    }

    fn read_memory_u8(&self, offset: u64) -> u8 {
        self.mem.read_memory_u8(offset)
    }

    fn write_memory(&mut self, offset: u64, buffer: &[u8]) {
        self.mem.write_memory(offset, buffer)
    }
}
