pub trait ContractErrorTrait {
    fn error_type(&self) -> &'static str;
    fn wrap(&self) -> String;
}

pub fn check_contract_error_trait<T: ContractErrorTrait>(_: &T) {}

#[crate::near(serializers = [json], inside_nearsdk = true)]
#[derive(Debug)]
pub struct BaseError {
    pub error: String,
}

impl ContractErrorTrait for BaseError {
    fn error_type(&self) -> &'static str {
        "BASE_ERROR"
    }

    fn wrap(&self) -> String {
        self.error.clone()
    }
}

impl From<BaseError> for String {
    fn from(value: BaseError) -> Self {
        value.error
    }
}

pub fn wrap_error<T: ContractErrorTrait>(error: T) -> String {
    error.wrap()
}

/// Shared implementation for wrapping errors into JSON format.
/// This function is used by generated `wrap()` methods to avoid code duplication to reduce wasm size.
#[inline(never)]
pub fn wrap_impl(error_type: &str, type_name: &str, info: &str) -> String {
    format!(
        r#"{{"error":{{"name":"{}","cause":{{"name":"{}","info":{}}}}}}}"#,
        error_type, type_name, info
    )
}
