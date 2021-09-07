#[cfg(not(target_arch = "wasm32"))]
pub use near_vm_logic::types::{PromiseResult as VmPromiseResult, ReturnData};

//* Types from near_vm_logic
pub type PromiseIndex = u64;
pub type ReceiptIndex = u64;
pub type IteratorIndex = u64;

/// When there is a callback attached to one or more contract calls the execution results of these
/// calls are available to the contract invoked through the callback.
#[derive(Debug, PartialEq)]
pub enum PromiseResult {
    /// Current version of the protocol never returns `PromiseResult::NotReady`.
    NotReady,
    Successful(Vec<u8>),
    Failed,
}

#[cfg(not(target_arch = "wasm32"))]
impl From<PromiseResult> for VmPromiseResult {
    fn from(p: PromiseResult) -> Self {
        match p {
            PromiseResult::NotReady => Self::NotReady,
            PromiseResult::Successful(v) => Self::Successful(v),
            PromiseResult::Failed => Self::Failed,
        }
    }
}

/// All error variants which can occur with promise results.
#[non_exhaustive]
pub enum PromiseError {
    /// Promise result failed.
    Failed,
    /// Current version of the protocol never returns this variant.
    NotReady,
}
