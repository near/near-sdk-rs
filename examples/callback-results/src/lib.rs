use near_sdk::require;
use near_sdk::{Promise, PromiseError, env, near};

const A_VALUE: u8 = 8;

#[near(contract_state)]
pub struct Callback;

#[near]
impl Callback {
    /// Returns a static string if `fail` is false, panicking otherwise.
    #[private]
    pub fn b(fail: bool) -> &'static str {
        if fail {
            env::panic_str("failed within function b");
        }
        "Some string"
    }

    /// Panics if value is 0, returns the value passed in otherwise.
    #[private]
    pub fn c(value: u8) -> u8 {
        require!(value > 0, "Value must be positive");
        value
    }

    /// Panics if value is 0.
    #[private]
    pub fn d(value: u8) {
        require!(value > 0, "Value must be positive");
    }

    /// Calls function c with a value that will always succeed
    pub fn a() -> Promise {
        Self::ext_self().c(A_VALUE)
    }

    /// Receives the callbacks from the other promises called.
    #[private]
    pub fn handle_callbacks(
        #[callback_unwrap] a: u8,
        #[callback_result(max_bytes = "\"Some string\"".len())] b: Result<String, PromiseError>,
        #[callback_result] c: Result<u8, PromiseError>,
        #[callback_result] d: Result<(), PromiseError>,
    ) -> (bool, bool, bool) {
        require!(a == A_VALUE, "Promise returned incorrect value");
        if let Ok(s) = b.as_ref() {
            require!(s == "Some string");
        }
        (b.is_err(), c.is_err(), d.is_err())
    }

    /// Call functions a, b, c, and d asynchronously and handle results with `handle_callbacks`.
    pub fn call_all(fail_b: bool, c_value: u8, d_value: u8) -> Promise {
        Self::ext_self()
            .a()
            .and(Self::ext_self().b(fail_b))
            .and(Self::ext_self().c(c_value))
            .and(Self::ext_self().d(d_value))
            .then(Self::ext_self().handle_callbacks())
    }

    /// Receives the callbacks from the other promises called.
    /// Used by the `call_all_reverse` test case to check that the order of
    /// `callback_unwrap`/`callback_result` arguments doesn't change the result.
    #[private]
    pub fn handle_callbacks_reverse(
        #[callback_result] b: Result<String, PromiseError>,
        #[callback_result] c: Result<u8, PromiseError>,
        #[callback_result] d: Result<(), PromiseError>,
        #[callback_unwrap] a: u8,
    ) -> (bool, bool, bool) {
        require!(a == A_VALUE, "Promise returned incorrect value");
        if let Ok(s) = b.as_ref() {
            require!(s == "Some string");
        }
        (b.is_err(), c.is_err(), d.is_err())
    }

    /// Call functions a, b, c, and d asynchronously and handle results with `handle_callbacks_reverse`.
    pub fn call_all_reverse(fail_b: bool, c_value: u8, d_value: u8) -> Promise {
        Self::ext_self()
            .b(fail_b)
            .and(Self::ext_self().c(c_value))
            .and(Self::ext_self().d(d_value))
            .and(Self::ext_self().a())
            .then(Self::ext_self().handle_callbacks_reverse())
    }
}
