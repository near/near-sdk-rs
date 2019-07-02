#[test]
fn tests() {
    let t = trybuild::TestCases::new();
    t.pass("compilation_tests/regular.rs");
    t.pass("compilation_tests/complex.rs");
    t.compile_fail("compilation_tests/impl_generic.rs");
    t.compile_fail("compilation_tests/method_generic.rs");
    t.compile_fail("compilation_tests/bad_argument.rs");
    t.compile_fail("compilation_tests/bad_return_type.rs");
    t.pass("compilation_tests/references.rs");
}
