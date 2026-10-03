/// `near-sys` declares the host-function ABI; ABI generation must define all of it,
/// or Windows linking fails when a host function is added without a stub.
/// Keep this outside the stub module: `unit-testing` disables that module.
#[test]
fn stubs_cover_every_near_sys_host_function() {
    let declarations = include_str!("../../near-sys/src/lib.rs");
    let stubs = include_str!("../src/abi_host_stubs.rs");

    let missing: Vec<&str> = declarations
        .lines()
        .filter_map(|line| line.trim().strip_prefix("pub fn "))
        .filter_map(|rest| rest.split(['(', '<']).next())
        .filter(|name| !stubs.contains(&format!("extern \"C-unwind\" fn {name}(")))
        .collect();

    assert!(
        missing.is_empty(),
        "near-sys host functions with no stub in abi_host_stubs.rs: {missing:?}"
    );
}
