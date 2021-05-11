// TODO restrict to non-wasm32
pub use near_vm_logic::types::{PromiseResult, ReturnData};

//* Types from near_vm_logic

pub type PublicKey = Vec<u8>;
pub type PromiseIndex = u64;
pub type ReceiptIndex = u64;
pub type IteratorIndex = u64;
