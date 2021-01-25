#[rustversion::stable]
#[test]
fn compilation_tests() {
    let t = trybuild::TestCases::new();
    t.pass("compilation_tests/regular.rs");
    t.pass("compilation_tests/private.rs");
    t.pass("compilation_tests/trait_impl.rs");
    t.pass("compilation_tests/metadata.rs");
    t.compile_fail("compilation_tests/metadata_invalid_rust.rs");
    t.pass("compilation_tests/complex.rs");
    t.compile_fail("compilation_tests/impl_generic.rs");
    t.compile_fail("compilation_tests/bad_argument.rs");
    t.pass("compilation_tests/references.rs");
    t.pass("compilation_tests/init_function.rs");
    t.pass("compilation_tests/no_default.rs");
    t.pass("compilation_tests/lifetime_method.rs");
    t.pass("compilation_tests/cond_compilation.rs");
    t.compile_fail("compilation_tests/payable_view.rs");
}
