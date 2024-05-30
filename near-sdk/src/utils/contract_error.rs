pub trait ContractErrorTrait {
    fn error_type(&self) -> &'static str;
    fn wrap(&self) -> String;
}

pub fn check_contract_error_trait<T: ContractErrorTrait>(_: &T) {}

#[crate::contract_error(inside_nearsdk)]
pub struct BaseError {
    pub error: serde_json::Value,
}

pub fn wrap_error<T: ContractErrorTrait>(error: T) -> String {
    error.wrap()
}