/// Enables contract runtime to panic with the given type. Any error type used in conjunction
/// with `#[handle_result]` has to implement this trait.
///
/// Example:
/// ```no_run
/// use near_sdk::{FunctionError, near};
///
/// enum Error {
///     NotFound,
///     Unexpected { message: String },
/// }
///
/// impl FunctionError for Error {
///     fn panic(&self) -> ! {
///         match self {
///             Error::NotFound =>
///                 near_sdk::env::panic_str("not found"),
///             Error::Unexpected { message } =>
///                 near_sdk::env::panic_str(&format!("unexpected error: {}", message))
///         }
///     }
/// }
///
/// #[near(contract_state)]
/// #[derive(Default)]
/// pub struct Contract;
///
/// #[near]
/// impl Contract {
///     // if the Error does not implement FunctionError, the following will not compile with #[handle_result]
///     #[handle_result]
///     pub fn set(&self, value: String) -> Result<String, Error> {
///         Err(Error::NotFound)
///     }
/// }
/// ```
pub trait FunctionError {
    fn panic(&self) -> !;
}

impl<T> FunctionError for T
where
    T: AsRef<str>,
{
    fn panic(&self) -> ! {
        crate::env::panic_str(self.as_ref())
    }
}

/// A simple type used in conjunction with [FunctionError] representing that the function should
/// abort without a custom message.
///
/// ```
/// use near_sdk::{Abort, near};
///
/// #[near(contract_state)]
/// #[derive(Default)]
/// pub struct Contract;
///
/// #[near]
/// impl Contract {
///     #[handle_result]
///     pub fn foo(&self, text: &str) -> Result<String, Abort> {
///         if text == "success" {
///             Ok("success".to_string())
///         } else {
///             Err(Abort)
///         }
///     }
/// }
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Abort;

impl std::fmt::Display for Abort {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "transaction aborted")
    }
}

impl FunctionError for Abort {
    fn panic(&self) -> ! {
        crate::env::abort()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_abort_display() {
        let abort = Abort;
        assert_eq!(abort.to_string(), "transaction aborted");
    }

    #[test]
    fn test_abort_debug() {
        let abort = Abort;
        assert_eq!(format!("{:?}", abort), "Abort");
    }

    #[test]
    fn test_abort_clone_copy() {
        let abort = Abort;
        // Deliberately test Clone on a Copy type
        #[allow(clippy::clone_on_copy)]
        let cloned = abort.clone();
        let copied = abort; // Copy
        assert_eq!(abort, cloned);
        assert_eq!(abort, copied);
    }

    #[test]
    fn test_abort_eq() {
        assert_eq!(Abort, Abort);
    }

    #[test]
    fn test_abort_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(Abort);
        assert!(set.contains(&Abort));
    }
}
