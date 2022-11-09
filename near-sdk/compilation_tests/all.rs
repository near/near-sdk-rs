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
    t.pass("compilation_tests/lifetime_method.rs");
    t.pass("compilation_tests/cond_compilation.rs");
    t.compile_fail("compilation_tests/payable_view.rs");
    t.pass("compilation_tests/borsh_storage_key.rs");
    t.pass("compilation_tests/borsh_storage_key_generics.rs");
    t.pass("compilation_tests/function_error.rs");
    t.pass("compilation_tests/enum_near_bindgen.rs");
    t.pass("compilation_tests/schema_derive.rs");
    t.compile_fail("compilation_tests/schema_derive_invalids.rs");
}
