use near_sdk_macros::{contract_error, near};

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


#[near(inside_nearsdk, serializers = [json])]
struct ErrorWrapper<T> {
    name: String,
    cause: ErrorCause<T>,
}

#[near(inside_nearsdk, serializers = [json])]
struct ErrorCause<T> {
    name: String,
    info: T
}

pub fn wrap_error<T>(error: T) -> String where T: serde::Serialize + crate::ContractErrorTrait {
    serde_json::json! {
        { "error" : ErrorWrapper {
            name: String::from(error.error_type()),
            cause: ErrorCause {
                name: std::any::type_name::<T>().to_string(),
                info: error
            }
        } }
    }.to_string()
}