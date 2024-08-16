#[rustversion::stable]
#[test]
fn compilation_tests() {
    let t = trybuild::TestCases::new();
    t.compile_fail("compilation_tests/invalid_arg_pat.rs");
    t.pass("compilation_tests/regular.rs");
    t.pass("compilation_tests/private.rs");
    t.pass("compilation_tests/trait_impl.rs");
    t.compile_fail("compilation_tests/bad_argument.rs");
    t.pass("compilation_tests/complex.rs");
    t.compile_fail("compilation_tests/impl_generic.rs");
    t.pass("compilation_tests/references.rs");
    t.pass("compilation_tests/init_function.rs");
    t.pass("compilation_tests/init_ignore_state.rs");
    t.pass("compilation_tests/no_default.rs");
    // TODO: unignore upon resolution of https://github.com/near/near-sdk-rs/issues/1211
    // t.pass("compilation_tests/lifetime_method_result.rs");
    t.pass("compilation_tests/lifetime_method.rs");
    t.pass("compilation_tests/cond_compilation.rs");
    t.compile_fail("compilation_tests/payable_view.rs");
    t.pass("compilation_tests/borsh_storage_key.rs");
    t.pass("compilation_tests/borsh_storage_key_generics.rs");
    t.pass("compilation_tests/function_error.rs");
    t.pass("compilation_tests/enum_near_bindgen.rs");
    t.pass("compilation_tests/schema_derive.rs");

    if rustversion::cfg!(since(1.80)) && std::env::consts::OS == "linux" {
        // The compilation error output has slightly changed in 1.7x and 1.8x and between platforms,
        // so we snapshoted this single version
        t.compile_fail("compilation_tests/schema_derive_invalids.rs");
    }
    t.compile_fail("compilation_tests/generic_function.rs");
    t.compile_fail("compilation_tests/generic_const_function.rs");
    t.pass("compilation_tests/self_support.rs");
    t.pass("compilation_tests/private_init_method.rs");
    t.compile_fail("compilation_tests/self_forbidden_in_non_init_fn_return.rs");
    t.compile_fail("compilation_tests/self_forbidden_in_non_init_fn_arg.rs");
    t.pass("compilation_tests/handle_result_alias.rs");
    t.pass("compilation_tests/contract_metadata.rs");
    t.compile_fail("compilation_tests/contract_metadata_fn_name.rs");
    t.pass("compilation_tests/contract_metadata_bindgen.rs");
    t.pass("compilation_tests/types.rs");
}
