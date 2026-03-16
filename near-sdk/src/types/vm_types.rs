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
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum PromiseError {
    /// Promise result failed.
    Failed,
    /// Promise succeeded but result length exceeded the limit.
    TooLong(
        /// The length (in bytes) of the result occurred
        usize,
    ),
}

impl std::fmt::Display for PromiseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PromiseError::Failed => write!(f, "promise result failed"),
            PromiseError::TooLong(len) => {
                write!(f, "promise result too long: {len} bytes")
            }
        }
    }
}

impl std::error::Error for PromiseError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_promise_error_display_failed() {
        let err = PromiseError::Failed;
        assert_eq!(err.to_string(), "promise result failed");
    }

    #[test]
    fn test_promise_error_display_too_long() {
        let err = PromiseError::TooLong(1024);
        assert_eq!(err.to_string(), "promise result too long: 1024 bytes");
    }

    #[test]
    fn test_promise_error_implements_std_error() {
        let err = PromiseError::Failed;
        // Verify it can be used as a &dyn std::error::Error
        let _: &dyn std::error::Error = &err;
    }

    #[test]
    fn test_promise_error_clone() {
        let err = PromiseError::TooLong(512);
        let cloned = err.clone();
        assert_eq!(err, cloned);
    }

    #[test]
    fn test_promise_error_debug() {
        let err = PromiseError::Failed;
        assert_eq!(format!("{:?}", err), "Failed");

        let err = PromiseError::TooLong(256);
        assert_eq!(format!("{:?}", err), "TooLong(256)");
    }
}
