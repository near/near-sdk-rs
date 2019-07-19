//! Test `near_bindgen::collections::Map` implementation.
#[macro_use]
mod common;
mod mock_host;
use wasm::executor::execute;
use wasm::types::Config;

#[test]
fn test_iter() {
    let code = common::contract_code();
    let context = common::runtime_context();
    let config = Config::default();
    let mut ext = mock_host::KVExternal::default();

    let result = execute(&code, b"map_to_vec", &[], &[], &mut ext, &config, &context).unwrap();
    check_result!(result, [[0, 10], [1, 11], [2, 12], [3, 13], [4, 14]]);
}

#[test]
fn test_empty() {
    let code = common::contract_code();
    let context = common::runtime_context();
    let config = Config::default();
    let mut ext = mock_host::KVExternal::default();
    execute(&code, b"map_clear", &[], &[], &mut ext, &config, &context).unwrap();

    let result = execute(&code, b"map_to_vec", &[], &[], &mut ext, &config, &context).unwrap();
    check_result_str!(result, "[]");
}

#[test]
fn test_pop() {
    let code = common::contract_code();
    let context = common::runtime_context();
    let config = Config::default();
    let mut ext = mock_host::KVExternal::default();
    execute(&code, b"map_remove", b"{\"key\": 0}", &[], &mut ext, &config, &context).unwrap();

    let result = execute(&code, b"map_to_vec", &[], &[], &mut ext, &config, &context).unwrap();
    check_result!(result, [[1, 11], [2, 12], [3, 13], [4, 14]]);
    for i in 1..5 {
        let result = execute(
            &code,
            b"map_remove",
            format!("{{\"key\": {} }}", i).as_bytes(),
            &[],
            &mut ext,
            &config,
            &context,
        )
        .unwrap();
        let expected_value = 10 + i;
        check_result!(result, expected_value);
    }
    let result = execute(&code, b"map_to_vec", &[], &[], &mut ext, &config, &context).unwrap();
    check_result_str!(result, "[]");
}

#[test]
fn test_insert() {
    let code = common::contract_code();
    let context = common::runtime_context();
    let config = Config::default();
    let mut ext = mock_host::KVExternal::default();

    let result = execute(
        &code,
        b"map_insert",
        b"{\"key\": 4, \"value\": 200}",
        &[],
        &mut ext,
        &config,
        &context,
    )
    .unwrap();
    check_result!(result, 14);
    let result = execute(&code, b"map_to_vec", &[], &[], &mut ext, &config, &context).unwrap();
    check_result!(result, [[0, 10], [1, 11], [2, 12], [3, 13], [4, 200]]);
    let result = execute(
        &code,
        b"map_insert",
        b"{\"key\": 5, \"value\": 100}",
        &[],
        &mut ext,
        &config,
        &context,
    )
    .unwrap();
    check_result_str!(result, "null");

    execute(&code, b"map_clear", &[], &[], &mut ext, &config, &context).unwrap();
    let result = execute(
        &code,
        b"map_insert",
        b"{\"key\": 0, \"value\": 0}",
        &[],
        &mut ext,
        &config,
        &context,
    )
    .unwrap();
    check_result_str!(result, "null");
    let result = execute(&code, b"map_to_vec", &[], &[], &mut ext, &config, &context).unwrap();
    check_result!(result, [[0, 0]]);
}
