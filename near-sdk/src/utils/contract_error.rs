pub trait ContractErrorTrait {
    fn error_type(&self) -> &'static str;
}

pub fn check_contract_error_trait<T: ContractErrorTrait>(_: &T) {}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct BaseError {
    pub error: serde_json::Value,
}

impl ContractErrorTrait for BaseError {
    fn error_type(&self) -> &'static str {
        "CUSTOM_CONTRACT_ERROR"
    }
}
