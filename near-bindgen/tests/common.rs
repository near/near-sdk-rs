use wasm::types::{ContractCode, RuntimeContext};

/// Create context for the `Runtime` with some default values.
pub fn runtime_context() -> RuntimeContext {
    RuntimeContext::new(
        0,
        1_000_000_000,
        &"alice.near".to_string(),
        &"bob".to_string(),
        0,
        123,
        b"yolo".to_vec(),
        false,
    )
}

/// Get the code of the contract.
pub fn contract_code() -> ContractCode {
    let data = include_bytes!("./res/test_contract.wasm");
    ContractCode::new(data.to_vec())
}
