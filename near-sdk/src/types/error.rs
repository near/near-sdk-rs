/// Enables contract runtime to panic with the given type. Any error type used in conjunction
/// with `#[handle_result]` has to implement this trait.
///
/// ```
/// use near_sdk::FunctionError;
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
