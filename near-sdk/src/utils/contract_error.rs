pub trait ContractErrorTrait {}

pub fn check_contract_error_trait<T: ContractErrorTrait>(_: &T) {}
