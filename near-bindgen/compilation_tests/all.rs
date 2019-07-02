#[test]
fn tests() {
    let t = trybuild::TestCases::new();
    t.pass("compilation_tests/regular.rs");
    t.pass("compilation_tests/complex.rs");
    t.compile_fail("compilation_tests/impl_generic.rs");
}
