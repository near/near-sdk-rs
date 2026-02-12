use std::borrow::Cow;

use near_sdk_macros::contract_error;

#[contract_error(inside_nearsdk, sdk)]
pub struct DepositNotAccepted<'a> {
    pub method: Cow<'a, str>,
}

impl DepositNotAccepted<'_> {
    pub fn new(method: &str) -> Self {
        Self { method: Cow::Owned(method.to_string()) }
    }
}

#[contract_error(inside_nearsdk, sdk)]
pub struct ContractNotInitialized {}

#[contract_error(inside_nearsdk, sdk)]
pub struct ContractAlreadyInitialized {}

#[contract_error(inside_nearsdk, sdk)]
pub struct CallbackComputationUnsuccessful {
    pub index: u64,
}

impl CallbackComputationUnsuccessful {
    pub fn new(index: u64) -> Self {
        Self { index }
    }
}

#[contract_error(inside_nearsdk, sdk)]
pub struct ActionInJointPromise<'a> {
    pub message: Cow<'a, str>,
}

impl ActionInJointPromise<'_> {
    pub fn new() -> Self {
        Self { message: Cow::Borrowed("Cannot add action to a joint promise.") }
    }
}

impl Default for ActionInJointPromise<'_> {
    fn default() -> Self {
        Self::new()
    }
}

#[contract_error(inside_nearsdk, sdk)]
pub struct PromiseAlreadyScheduled<'a> {
    pub message: Cow<'a, str>,
}

impl PromiseAlreadyScheduled<'_> {
    pub fn new() -> Self {
        Self {
            message: Cow::Borrowed(
                "Cannot callback promise which is already scheduled after another",
            ),
        }
    }
}

impl Default for PromiseAlreadyScheduled<'_> {
    fn default() -> Self {
        Self::new()
    }
}

#[contract_error(inside_nearsdk, sdk)]
pub struct CallbackYieldPromise<'a> {
    pub message: Cow<'a, str>,
}

impl CallbackYieldPromise<'_> {
    pub fn new() -> Self {
        Self { message: Cow::Borrowed("Cannot callback yielded promise") }
    }
}

impl Default for CallbackYieldPromise<'_> {
    fn default() -> Self {
        Self::new()
    }
}

#[contract_error(inside_nearsdk, sdk)]
pub struct CallbackJointPromise<'a> {
    pub message: Cow<'a, str>,
}

impl CallbackJointPromise<'_> {
    pub fn new() -> Self {
        Self { message: Cow::Borrowed("Cannot callback joint promise") }
    }
}

impl Default for CallbackJointPromise<'_> {
    fn default() -> Self {
        Self::new()
    }
}

#[contract_error(inside_nearsdk, sdk)]
pub struct PrivateMethod<'a> {
    pub method_name: Cow<'a, str>,
}

impl PrivateMethod<'_> {
    pub fn new(method_name: &str) -> Self {
        Self { method_name: Cow::Owned(method_name.to_string()) }
    }
}

#[contract_error(inside_nearsdk, sdk)]
pub struct BorshSerializeError<'a> {
    subject: Cow<'a, str>,
}

impl BorshSerializeError<'_> {
    pub fn new(subject: &str) -> Self {
        Self { subject: Cow::Owned(subject.to_string()) }
    }
}

#[contract_error(inside_nearsdk, sdk)]
pub struct BorshDeserializeError<'a> {
    subject: Cow<'a, str>,
}

impl BorshDeserializeError<'_> {
    pub fn new(subject: &str) -> Self {
        Self { subject: Cow::Owned(subject.to_string()) }
    }
}

#[contract_error(inside_nearsdk, sdk)]
pub struct InvalidTreeMapRange {}

#[contract_error(inside_nearsdk, sdk)]
pub struct InconsistentCollectionState<'a> {
    pub message: Cow<'a, str>,
}

impl InconsistentCollectionState<'_> {
    pub fn new() -> Self {
        Self {
            message: Cow::Borrowed(
                "The collection is an inconsistent state. Did previous smart contract execution terminate unexpectedly?",
            ),
        }
    }
}

impl Default for InconsistentCollectionState<'_> {
    fn default() -> Self {
        Self::new()
    }
}

#[contract_error(inside_nearsdk, sdk)]
pub struct IndexOutOfBounds {}

#[contract_error(inside_nearsdk, sdk)]
pub struct KeyNotFound {}

#[contract_error(inside_nearsdk, sdk)]
pub struct RegisterEmpty<'a> {
    pub message: Cow<'a, str>,
}

impl RegisterEmpty<'_> {
    pub fn new() -> Self {
        Self {
            message: Cow::Borrowed(
                "Register was expected to have data because we just wrote it into it.",
            ),
        }
    }
}

impl Default for RegisterEmpty<'_> {
    fn default() -> Self {
        Self::new()
    }
}
