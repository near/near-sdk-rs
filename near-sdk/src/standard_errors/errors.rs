use near_sdk_macros::contract_error;

#[contract_error]
pub struct InvalidArgument {
    message: String,
}