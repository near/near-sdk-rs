pub trait ContractErrorTrait {
    fn error_type(&self) -> &'static str;
}

pub fn check_contract_error_trait<T: ContractErrorTrait>(_: &T) {}
