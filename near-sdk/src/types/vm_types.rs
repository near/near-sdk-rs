#[cfg(all(not(target_arch = "wasm32"), feature = "unit-testing"))]
pub use near_vm_runner::logic::types::{PromiseResult as VmPromiseResult, ReturnData};

//* Types from near_vm_logic
/// Promise index that is computed only once. It is an internal index that identifies a specific promise (or a sequence of promises) created during the execution of a smart contract.
/// Returned by [`promise_create`](crate::env::promise_create) and can be used to refer this promise in `promise_then`, `promise_batch_create`, and other functions.
/// Example:
/// ```no_run
/// use near_sdk::{env, Gas, AccountId, NearToken};
/// use std::str::FromStr;
///
/// let promise_id = env::promise_create(
///     AccountId::from_str("a.near").unwrap(), "new", b"{}", NearToken::from_yoctonear(0),
///     Gas::from_tgas(1)
/// );
/// env::promise_then(
///     promise_id, AccountId::from_str("b.near").unwrap(), "callback", b"{}", NearToken::from_yoctonear(0),
///     Gas::from_tgas(1)
/// );
/// ```
#[derive(Debug, Eq, PartialEq, PartialOrd, Ord, Hash, Copy, Clone)]
pub struct PromiseIndex(pub(crate) u64);

/// An index of Receipt to append an action
#[deprecated(since = "4.1.0", note = "type not used within SDK, use u64 directly or another alias")]
pub type ReceiptIndex = u64;
#[deprecated(since = "4.1.0", note = "type not used within SDK, use u64 directly or another alias")]
pub type IteratorIndex = u64;

/// When there is a callback attached to one or more contract calls the execution results of these
/// calls are available to the contract invoked through the callback.
#[derive(Debug, PartialEq, Eq)]
pub enum PromiseResult {
    Successful(Vec<u8>),
    Failed,
}

#[cfg(all(not(target_arch = "wasm32"), feature = "unit-testing"))]
impl From<PromiseResult> for VmPromiseResult {
    fn from(p: PromiseResult) -> Self {
        match p {
            PromiseResult::Successful(v) => Self::Successful(v.into_boxed_slice().into()),
            PromiseResult::Failed => Self::Failed,
        }
    }
}

/// All error variants which can occur with promise results.
#[non_exhaustive]
#[derive(Debug, PartialEq, Eq)]
pub enum PromiseError {
    /// Promise result failed.
    Failed,
    /// Promise succeeded but result length exceeded the limit.
    TooLong(
        /// The length (in bytes) of the result occurred
        usize,
    ),
}
