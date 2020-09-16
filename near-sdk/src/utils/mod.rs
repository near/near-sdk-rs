use crate::Balance;

#[macro_export]
macro_rules! log {
    ($arg:tt) => {
        crate::env::log($arg.as_bytes())
    };
    ($($arg:tt)*) => {
        crate::env::log(format!($($arg)*).as_bytes())
    };
}

/// Represents 1 yocto-NEAR token
pub const YOCTO_NEAR: Balance = 1;

/// Represents 1 micro-NEAR token or 1_000_000_000_000_000 yocto-NEAR tokens (10**15).
pub const NANO_NEAR: Balance = 1_000_000_000_000_000;

/// Represents 1 micro-NEAR token or 1_000_000_000_000_000_000 yocto-NEAR tokens (10**18).
pub const MICRO_NEAR: Balance = 1_000_000_000_000_000_000;

/// Represents 1 milli-NEAR token or 1_000_000_000_000_000_000_000 yocto-NEAR tokens (10**21).
pub const MILLI_NEAR: Balance = 1_000_000_000_000_000_000_000;

/// Represents 1 NEAR token or 1_000_000_000_000_000_000_000_000 yocto-NEAR tokens (10**24).
pub const NEAR: Balance = 1_000_000_000_000_000_000_000_000;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::{get_logs, test_env};

    #[test]
    fn test_token_denominations() {
        assert_eq!(YOCTO_NEAR * 10u128.pow(24), NEAR);
        assert_eq!(MILLI_NEAR * 1000, NEAR);
        assert_eq!(MICRO_NEAR * 1000, MILLI_NEAR);
        assert_eq!(NANO_NEAR * 1000, MICRO_NEAR);
    }

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
