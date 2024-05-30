use near_sdk_macros::contract_error;

#[contract_error(inside_nearsdk)]
pub struct InvalidArgument {
    pub message: String,
}

#[contract_error(inside_nearsdk, sdk)]
pub struct ContractNotInitialized {
    pub message: String,
}

impl ContractNotInitialized {
    pub fn new() -> Self {
        Self {
            message: "The contract is not initialized".to_string(),
        }
    }
}

#[contract_error(inside_nearsdk)]
pub struct RequireFailed {
    pub message: String,
}

impl RequireFailed {
    pub fn new() -> Self {
        Self {
            message: "require! assertion failed".to_string(),
        }
    }
}