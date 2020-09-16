#[macro_export]
macro_rules! log {
    ($arg:tt) => {
        crate::env::log($arg.as_bytes())
    };
    ($($arg:tt)*) => {
        crate::env::log(format!($($arg)*).as_bytes())
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
