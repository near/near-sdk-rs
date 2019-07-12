//! Test `near_bindgen::collections::Set` implementation.
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

    let result = execute(&code, b"set_to_vec", &[], &[], &mut ext, &config, &context).unwrap();
    check_result!(result, [0, 1, 2, 3, 4]);
}

#[test]
fn test_empty() {
    let code = common::contract_code();
    let context = common::runtime_context();
    let config = Config::default();
    let mut ext = mock_host::KVExternal::default();
    execute(&code, b"set_clear", &[], &[], &mut ext, &config, &context).unwrap();

    let result = execute(&code, b"set_to_vec", &[], &[], &mut ext, &config, &context).unwrap();
    check_result_str!(result, "[]");
}

#[test]
fn test_remove() {
    let code = common::contract_code();
    let context = common::runtime_context();
    let config = Config::default();
    let mut ext = mock_host::KVExternal::default();
    execute(&code, b"set_remove", b"{\"value\": 0}", &[], &mut ext, &config, &context).unwrap();

    let result = execute(&code, b"set_to_vec", &[], &[], &mut ext, &config, &context).unwrap();
    check_result!(result, [1, 2, 3, 4]);
    for i in 1..5 {
        let result = execute(
            &code,
            b"set_remove",
            format!("{{\"value\": {} }}", i).as_bytes(),
            &[],
            &mut ext,
            &config,
            &context,
        )
        .unwrap();
        check_result!(result, true);
    }
    let result = execute(&code, b"set_to_vec", &[], &[], &mut ext, &config, &context).unwrap();
    check_result_str!(result, "[]");
}

#[test]
fn test_insert() {
    let code = common::contract_code();
    let context = common::runtime_context();
    let config = Config::default();
    let mut ext = mock_host::KVExternal::default();

    let result =
        execute(&code, b"set_insert", b"{\"value\": 4}", &[], &mut ext, &config, &context).unwrap();
    check_result!(result, false);
    let result = execute(&code, b"set_to_vec", &[], &[], &mut ext, &config, &context).unwrap();
    check_result!(result, [0, 1, 2, 3, 4]);
    let result =
        execute(&code, b"set_insert", b"{\"value\": 5}", &[], &mut ext, &config, &context).unwrap();
    check_result!(result, true);

    execute(&code, b"set_clear", &[], &[], &mut ext, &config, &context).unwrap();
    let result =
        execute(&code, b"set_insert", b"{\"value\": 0}", &[], &mut ext, &config, &context).unwrap();
    check_result!(result, true);
}
