use std::borrow::Cow;

use near_sdk_macros::contract_error;

#[contract_error(inside_nearsdk)]
pub struct InvalidArgument<'a> {
    pub message: Cow<'a, str>,
}

impl InvalidArgument<'_> {
    pub fn new(message: &str) -> Self {
        Self { message: Cow::Owned(message.to_string()) }
    }
}

#[contract_error(inside_nearsdk)]
pub struct InvalidContractState<'a> {
    pub message: Cow<'a, str>,
}

impl InvalidContractState<'_> {
    pub fn new(message: &str) -> Self {
        Self { message: Cow::Owned(message.to_string()) }
    }
}

#[contract_error(inside_nearsdk)]
pub struct PermissionDenied<'a> {
    message: Option<Cow<'a, str>>,
}

impl PermissionDenied<'_> {
    pub fn new(message: Option<&str>) -> Self {
        Self { message: message.map(|s| Cow::Owned(s.to_string())) }
    }
}

#[contract_error(inside_nearsdk)]
pub struct ContractUpgradeError<'a> {
    pub message: Cow<'a, str>,
}

impl ContractUpgradeError<'_> {
    pub fn new(message: &str) -> Self {
        Self { message: Cow::Owned(message.to_string()) }
    }
}

#[contract_error(inside_nearsdk)]
pub struct RequireFailed<'a> {
    pub message: Cow<'a, str>,
}

impl RequireFailed<'_> {
    pub fn new() -> Self {
        Self { message: Cow::Borrowed("require! assertion failed") }
    }

    pub fn new_from_message(message: String) -> Self {
        Self { message: Cow::Owned(message) }
    }
}

impl Default for RequireFailed<'_> {
    fn default() -> Self {
        Self::new()
    }
}

#[contract_error(inside_nearsdk)]
pub struct PromiseFailed<'a> {
    pub promise_index: Option<u64>,
    pub message: Option<Cow<'a, str>>,
}

impl PromiseFailed<'_> {
    pub fn new(promise_index: Option<u64>, message: Option<&str>) -> Self {
        Self { promise_index, message: message.map(|s| Cow::Owned(s.to_string())) }
    }
}

#[contract_error(inside_nearsdk)]
pub struct InvalidPromiseReturn<'a> {
    pub message: Cow<'a, str>,
}

impl InvalidPromiseReturn<'_> {
    pub fn new(message: &str) -> Self {
        Self { message: Cow::Owned(message.to_string()) }
    }
}

#[contract_error(inside_nearsdk)]
pub struct InsufficientBalance<'a> {
    message: Option<Cow<'a, str>>,
}

impl InsufficientBalance<'_> {
    pub fn new(message: Option<&str>) -> Self {
        Self { message: message.map(|s| Cow::Owned(s.to_string())) }
    }
}

// Note: We use InsufficientGas {} rather than unit type InsufficientGas;
// The latter serializes to null and OpenAPI spec 3.0 (for example progenitor uses it) doesn't support it. Though 3.1 does.
#[contract_error(inside_nearsdk)]
pub struct InsufficientGas {}

#[contract_error(inside_nearsdk)]
pub struct TotalSupplyOverflow {}

#[contract_error(inside_nearsdk)]
pub struct UnexpectedFailure<'a> {
    pub message: Cow<'a, str>,
}

#[contract_error(inside_nearsdk)]
pub struct ContractError<'a> {
    pub message: Cow<'a, str>,
}

impl ContractError<'_> {
    pub fn new(message: &str) -> Self {
        Self { message: Cow::Owned(message.to_string()) }
    }
}

#[contract_error(inside_nearsdk)]
pub struct InvalidHashLength {
    pub expected: usize,
}

impl InvalidHashLength {
    pub fn new(expected: usize) -> Self {
        Self { expected }
    }
}
