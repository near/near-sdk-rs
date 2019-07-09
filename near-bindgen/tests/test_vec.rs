//! Test `near_bindgen::collections::Vec` implementation.
mod common;
mod mock_host;
use serde_json::json;
use wasm::executor::execute;
use wasm::types::{Config, ReturnData};

#[test]
fn test_pop() {
    let code = common::contract_code();
    let context = common::runtime_context();
    let config = Config::default();
    let mut ext = mock_host::KVExternal::default();
    let method_name = b"pop";
    execute(&code, method_name, &[], &[], &mut ext, &config, &context).unwrap();

    let method_name = b"to_vec";
    let result = execute(&code, method_name, &[], &[], &mut ext, &config, &context).unwrap();
    match result.return_data {
        Ok(ReturnData::Value(v)) => {
            assert_eq!(json!([0, 1, 2, 3]).to_string(), String::from_utf8(v).unwrap())
        }
        _ => panic!(),
    }
}
