const SERIALIZED_STATE: &[u8] = b"STATE";

#[cfg(not(feature = "env_test"))]
pub struct NearContext {}

#[cfg(not(feature = "env_test"))]
pub fn read_state<T: serde::de::DeserializeOwned>() -> Option<T> {
    if !unsafe { near_bindgen::CONTEXT.storage_has_key(SERIALIZED_STATE) } {
        return None;
    }
    let data = near_bindgen::CONTEXT.storage_read(SERIALIZED_STATE);
    bincode::deserialize(&data).ok()
}

#[cfg(not(feature = "env_test"))]
pub fn write_state<T: serde::Serialize>(state: &T) {
    let data = bincode::serialize(state).unwrap();
    unsafe {
        near_bindgen::CONTEXT.storage_write(
            SERIALIZED_STATE,
            &data,
        );
    }
}

#[cfg(not(feature = "env_test"))]
impl near_bindgen::context::Context for NearContext {
    fn storage_write(&self, key: &[u8], value: &[u8]) {
        unsafe {
            sys::storage_write(key.len() as _, key.as_ptr(), value.len() as _, value.as_ptr())
        }
    }

    fn storage_iter(&self, prefix: &[u8]) -> u32 {
        unsafe { sys::storage_iter(prefix.len() as _, prefix.as_ptr()) }
    }

    fn storage_range(&self, start: &[u8], end: &[u8]) -> u32 {
        unsafe {
            sys::storage_range(start.len() as _, start.as_ptr(), end.len() as _, end.as_ptr())
        }
    }

    fn storage_iter_next(&self, iter_id: u32) -> bool {
        unsafe { sys::storage_iter_next(iter_id) != 0 }
    }

    fn storage_remove(&self, key: &[u8]) {
        unsafe { sys::storage_remove(key.len() as _, key.as_ptr()) }
    }

    fn storage_has_key(&self, key: &[u8]) -> bool {
        unsafe { sys::storage_has_key(key.len() as _, key.as_ptr()) }
    }

    fn result_count(&self) -> u32 {
        unsafe { sys::result_count() }
    }

    fn result_is_ok(&self, index: u32) -> bool {
        unsafe { sys::result_is_ok(index) }
    }

    fn return_value(&self, value: &[u8]) {
        unsafe { sys::return_value(value.len() as _, value.as_ptr()) }
    }

    fn return_promise(&self, promise_index: u32) {
        unsafe { sys::return_promise(promise_index) }
    }

    fn data_read(
        &self,
        data_type_index: u32,
        key_len: usize,
        key: u32,
        max_buf_len: usize,
        buf_ptr: *mut u8,
    ) -> usize {
        unsafe { sys::data_read(data_type_index, key_len, key, max_buf_len, buf_ptr) }
    }

    fn promise_create(
        &self,
        account_id: &[u8],
        method_name: &[u8],
        arguments: &[u8],
        amount: u64,
    ) -> u32 {
        unsafe {
            sys::promise_create(
                account_id.len() as _,
                account_id.as_ptr(),
                method_name.len() as _,
                method_name.as_ptr(),
                arguments.len() as _,
                arguments.as_ptr(),
                amount,
            )
        }
    }

    fn promise_then(
        &self,
        promise_index: u32,
        method_name: &[u8],
        arguments: &[u8],
        amount: u64,
    ) -> u32 {
        unsafe {
            sys::promise_then(
                promise_index,
                method_name.len() as _,
                method_name.as_ptr(),
                arguments.len() as _,
                arguments.as_ptr(),
                amount,
            )
        }
    }

    fn promise_and(&self, promise_index1: u32, promise_index2: u32) -> u32 {
        unsafe { sys::promise_and(promise_index1, promise_index2) }
    }

    fn frozen_balance(&self) -> u64 {
        unsafe { sys::frozen_balance() }
    }

    fn liquid_balance(&self) -> u64 {
        unsafe { sys::liquid_balance() }
    }

    fn deposit(&self, min_amount: u64, max_amount: u64) -> u64 {
        unsafe { sys::deposit(min_amount, max_amount) }
    }

    fn withdraw(&self, min_amount: u64, max_amount: u64) -> u64 {
        unsafe { sys::withdraw(min_amount, max_amount) }
    }

    fn received_amount(&self) -> u64 {
        unsafe { sys::received_amount() }
    }

    fn storage_usage(&self) -> u64 {
        unsafe { sys::storage_usage() }
    }

    fn assert(&self, expr: bool) {
        unsafe { sys::assert(expr) }
    }

    fn random_buf(&self, buf: &mut [u8]) {
        unsafe { sys::random_buf(buf.len() as _, buf.as_mut_ptr()) }
    }

    fn random32(&self) -> u32 {
        unsafe { sys::random32() }
    }

    fn block_index(&self) -> u64 {
        unsafe { sys::block_index() }
    }

    fn debug(&self, msg: &[u8]) {
        unsafe { sys::debug(msg.len() as _, msg.as_ptr()) }
    }
}
