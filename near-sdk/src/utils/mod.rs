use crate::AccountId;

#[macro_export]
macro_rules! log {
    ($arg:tt) => {
        crate::env::log($arg.as_bytes())
    };
    ($($arg:tt)*) => {
        crate::env::log(format!($($arg)*).as_bytes())
    };
}

#[derive(Debug)]
pub struct PendingContractTx {
    pub receiver_id: AccountId,
    pub method: String,
    pub args: Vec<u8>,
    pub is_view: bool,
}

impl PendingContractTx {
    pub fn new(receiver_id: &str, method: &str, args: serde_json::Value, is_view: bool) -> Self {
        Self {
            receiver_id: receiver_id.to_string(),
            method: method.to_string(),
            args: args.to_string().into_bytes(),
            is_view,
        }
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
