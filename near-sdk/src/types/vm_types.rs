#[cfg(not(target_arch = "wasm32"))]
pub use near_vm_logic::types::ReturnData;

//* Types from near_vm_logic

pub type PublicKey = Vec<u8>;
pub type PromiseIndex = u64;
pub type ReceiptIndex = u64;
pub type IteratorIndex = u64;

// TODO(austinabell): This is not a great solution, but there is no need for the connection to
// near_vm_logic and the serialize implementation is faulty. The type only needs to be kept as
// is below because it is used only in a mocked blockchain interface.
/// When there is a callback attached to one or more contract calls the execution results of these
/// calls are available to the contract invoked through the callback.
#[cfg(target_arch = "wasm32")]
#[derive(Debug, PartialEq)]
pub enum PromiseResult {
    /// Current version of the protocol never returns `PromiseResult::NotReady`.
    NotReady,
    Successful(Vec<u8>),
    Failed,
}

#[cfg(not(target_arch = "wasm32"))]
pub use near_vm_logic::types::PromiseResult;
