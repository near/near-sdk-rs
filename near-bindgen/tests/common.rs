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

macro_rules! check_result_str {
    ($result:ident, $json_str:expr) => {
        match $result.return_data {
            Ok(wasm::types::ReturnData::Value(v)) => {
                assert_eq!($json_str, String::from_utf8(v).unwrap())
            }
            _ => panic!(),
        }
    };
}

macro_rules! check_result {
    ($result:ident, $json_expr:expr) => {
        match $result.return_data {
            Ok(wasm::types::ReturnData::Value(v)) => {
                assert_eq!(serde_json::json!($json_expr).to_string(), String::from_utf8(v).unwrap())
            }
            _ => panic!(),
        }
    };
}
