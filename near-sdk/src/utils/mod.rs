pub(crate) mod storage_key_impl;

use crate::{env, AccountId, PromiseResult};

#[macro_export]
macro_rules! log {
    ($arg:expr) => {
        $crate::env::log($arg.as_ref())
    };
    ($($arg:tt)*) => {
        $crate::env::log(format!($($arg)*).as_str())
    };
}

/// Assert that predecessor_account_id == current_account_id, meaning contract called itself.
pub fn assert_self() {
    assert_eq!(env::predecessor_account_id(), env::current_account_id(), "Method is private");
}

/// Assert that 1 yoctoNEAR was attached.
pub fn assert_one_yocto() {
    assert_eq!(env::attached_deposit(), 1, "Requires attached deposit of exactly 1 yoctoNEAR")
}

/// Returns true if promise was successful.
/// Fails if called outside a callback that received 1 promise result.
pub fn is_promise_success() -> bool {
    promise_result_as_success().is_some()
}

/// Returns the result of the promise if successful. Otherwise returns None.
/// Fails if called outside a callback that received 1 promise result.
pub fn promise_result_as_success() -> Option<Vec<u8>> {
    assert_eq!(env::promise_results_count(), 1, "Contract expected a result on the callback");
    match env::promise_result(0) {
        PromiseResult::Successful(result) => Some(result),
        _ => None,
    }
}

/// Used in the simulation code generator from near_sdk.
#[derive(Debug)]
pub struct PendingContractTx {
    pub receiver_id: AccountId,
    pub method: String,
    pub args: Vec<u8>,
    pub is_view: bool,
}

impl PendingContractTx {
    pub fn new(receiver_id: &str, method: &str, args: serde_json::Value, is_view: bool) -> Self {
        PendingContractTx::new_from_bytes(
            receiver_id,
            method,
            args.to_string().into_bytes(),
            is_view,
        )
    }

    pub fn new_from_bytes(receiver_id: &str, method: &str, args: Vec<u8>, is_view: bool) -> Self {
        Self { receiver_id: receiver_id.to_string(), method: method.to_string(), args, is_view }
    }
}

/// Boilerplate for setting up allocator used in Wasm binary.
#[macro_export]
macro_rules! setup_alloc {
    () => {
        #[cfg(target_arch = "wasm32")]
        #[global_allocator]
        static ALLOC: near_sdk::wee_alloc::WeeAlloc<'_> = near_sdk::wee_alloc::WeeAlloc::INIT;
    };
}

#[cfg(test)]
mod tests {
    use crate::test_utils::{get_logs, test_env};

    #[test]
    fn test_log_simple() {
        test_env::setup();
        log!("hello");

        assert_eq!(get_logs(), vec!["hello".to_string()]);
    }

    #[test]
    fn test_log_format() {
        test_env::setup();
        log!("hello {} ({})", "user_name", 25);

        assert_eq!(get_logs(), vec!["hello user_name (25)".to_string()]);
    }
}
