//! Test `near_bindgen::collections::Vec` implementation.
#[macro_use]
mod common;
mod mock_host;
use wasm::executor::execute;
use wasm::types::Config;

#[test]
fn test_pop() {
    let code = common::contract_code();
    let context = common::runtime_context();
    let config = Config::default();
    let mut ext = mock_host::KVExternal::default();
    execute(&code, b"vec_pop", &[], &[], &mut ext, &config, &context).unwrap();

    let result = execute(&code, b"vec_to_vec", &[], &[], &mut ext, &config, &context).unwrap();
    check_result!(result, [0, 1, 2, 3]);
    for _ in 0..4 {
        execute(&code, b"vec_pop", &[], &[], &mut ext, &config, &context).unwrap();
    }
    let result = execute(&code, b"vec_to_vec", &[], &[], &mut ext, &config, &context).unwrap();
    check_result_str!(result, "[]");
}

#[test]
fn test_insert() {
    let code = common::contract_code();
    let context = common::runtime_context();
    let config = Config::default();
    let mut ext = mock_host::KVExternal::default();
    execute(
        &code,
        b"vec_insert",
        b"{\"index\": 2, \"element\": 10}",
        &[],
        &mut ext,
        &config,
        &context,
    )
    .unwrap();
    execute(
        &code,
        b"vec_insert",
        b"{\"index\": 0, \"element\": 20}",
        &[],
        &mut ext,
        &config,
        &context,
    )
    .unwrap();
    execute(
        &code,
        b"vec_insert",
        b"{\"index\": 7, \"element\": 30}",
        &[],
        &mut ext,
        &config,
        &context,
    )
    .unwrap();

    let result = execute(&code, b"vec_to_vec", &[], &[], &mut ext, &config, &context).unwrap();
    check_result!(result, [20, 0, 1, 10, 2, 3, 4, 30]);
}

#[test]
fn test_insert_empty() {
    let code = common::contract_code();
    let context = common::runtime_context();
    let config = Config::default();
    let mut ext = mock_host::KVExternal::default();
    execute(&code, b"vec_clear", &[], &[], &mut ext, &config, &context).unwrap();
    execute(
        &code,
        b"vec_insert",
        b"{\"index\": 0, \"element\": 1}",
        &[],
        &mut ext,
        &config,
        &context,
    )
    .unwrap();

    let result = execute(&code, b"vec_to_vec", &[], &[], &mut ext, &config, &context).unwrap();
    check_result!(result, [1]);
}

#[test]
fn test_remove() {
    let code = common::contract_code();
    let context = common::runtime_context();
    let config = Config::default();
    let mut ext = mock_host::KVExternal::default();
    execute(&code, b"vec_remove", b"{\"index\": 0}", &[], &mut ext, &config, &context).unwrap();
    execute(&code, b"vec_remove", b"{\"index\": 3}", &[], &mut ext, &config, &context).unwrap();
    execute(&code, b"vec_remove", b"{\"index\": 1}", &[], &mut ext, &config, &context).unwrap();
    execute(&code, b"vec_remove", b"{\"index\": 10}", &[], &mut ext, &config, &context).unwrap();

    let result = execute(&code, b"vec_to_vec", &[], &[], &mut ext, &config, &context).unwrap();
    check_result!(result, [1, 3]);

    // Remove until it is empty and then try removing a couple times more.
    for _ in 0..5 {
        execute(&code, b"vec_remove", b"{\"index\": 0}", &[], &mut ext, &config, &context).unwrap();
    }
}

#[test]
fn test_get_pop_first_last() {
    let code = common::contract_code();
    let context = common::runtime_context();
    let config = Config::default();
    let mut ext = mock_host::KVExternal::default();

    // Check get.
    let result =
        execute(&code, b"vec_get", b"{\"index\": 2}", &[], &mut ext, &config, &context).unwrap();
    check_result!(result, 2);

    // Check pop.
    execute(&code, b"vec_pop", &[], &[], &mut ext, &config, &context).unwrap();
    let result = execute(&code, b"vec_to_vec", &[], &[], &mut ext, &config, &context).unwrap();
    check_result!(result, [0, 1, 2, 3]);

    // Check first.
    let result = execute(&code, b"vec_first", &[], &[], &mut ext, &config, &context).unwrap();
    check_result!(result, 0);

    // Check last.
    let result = execute(&code, b"vec_last", &[], &[], &mut ext, &config, &context).unwrap();
    check_result!(result, 3);
}

#[test]
fn test_drain() {
    let code = common::contract_code();
    let context = common::runtime_context();
    let config = Config::default();
    let mut ext = mock_host::KVExternal::default();

    // Check drain.
    let result = execute(
        &code,
        b"vec_drain",
        b"{\"start\": 1, \"end\": 3}",
        &[],
        &mut ext,
        &config,
        &context,
    )
    .unwrap();
    check_result!(result, [1, 2]);

    // Check state after drain.
    let result = execute(&code, b"vec_to_vec", &[], &[], &mut ext, &config, &context).unwrap();
    check_result!(result, [0, 3, 4]);
}
