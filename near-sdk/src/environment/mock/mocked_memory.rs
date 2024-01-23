//!This file is to be removed once near-vm-logic is updated from version 0.17 and MockedMemory size can be customized
use near_vm_runner::logic::{MemSlice, MemoryLike};

#[derive(Default)]
pub struct MockedMemory {}

impl MemoryLike for MockedMemory {
    fn fits_memory(&self, _slice: MemSlice) -> Result<(), ()> {
        Ok(())
    }

    fn read_memory(&self, ptr: u64, buffer: &mut [u8]) -> Result<(), ()> {
        let src = unsafe { std::slice::from_raw_parts(ptr as *const u8, buffer.len()) };
        buffer.copy_from_slice(src);
        Ok(())
    }

    fn write_memory(&mut self, ptr: u64, buffer: &[u8]) -> Result<(), ()> {
        let dest = unsafe { std::slice::from_raw_parts_mut(ptr as *mut u8, buffer.len()) };
        dest.copy_from_slice(buffer);
        Ok(())
    }

    fn view_memory(&self, slice: MemSlice) -> Result<std::borrow::Cow<[u8]>, ()> {
        let src = unsafe { std::slice::from_raw_parts(slice.ptr as *const u8, slice.len as usize) };

        Ok(std::borrow::Cow::Borrowed(src))
    }
}
