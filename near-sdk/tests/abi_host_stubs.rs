/// `near-sys` declares the host-function ABI; ABI generation must define all of it,
/// or Windows linking fails when a host function is added without a stub.
/// Keep this outside the stub module: `unit-testing` disables that module.
#[test]
fn stubs_cover_every_near_sys_host_function() {
    let declarations = include_str!("../../near-sys/src/lib.rs");
    let stubs = include_str!("../src/abi_host_stubs.rs");

    let mut mismatched = Vec::new();
    for (name, declared) in signatures(declarations, "pub fn ", ';') {
        let stub = signatures(stubs, "extern \"C-unwind\" fn ", '{')
            .find(|(stub_name, _)| *stub_name == name)
            .map(|(_, sig)| sig);
        if stub.as_ref() != Some(&declared) {
            mismatched.push(format!("{name}: near-sys {declared:?}, stub {stub:?}"));
        }
    }

    assert!(
        mismatched.is_empty(),
        "abi_host_stubs.rs is missing or disagrees with these near-sys host functions:\n{}",
        mismatched.join("\n")
    );
}

/// Yields `(name, (parameter types, return type))` for every `{prefix}name(...)` item in
/// `source`, reading up to `end`. Parameter names are dropped, so `_ptr: u64` in a stub
/// matches `ptr: u64` in `near-sys`.
fn signatures<'a>(
    source: &'a str,
    prefix: &'a str,
    end: char,
) -> impl Iterator<Item = (&'a str, (Vec<String>, String))> + 'a {
    source.split(prefix).skip(1).filter_map(move |item| {
        let (name, rest) = item.split_once('(')?;
        let (params, rest) = rest.split_once(')')?;
        let params = params
            .split(',')
            .filter_map(|param| param.split_once(':'))
            .map(|(_, ty)| ty.split_whitespace().collect())
            .collect();
        let ret = rest.split(end).next()?;
        let ret = ret.trim().trim_start_matches("->").split_whitespace().collect();
        Some((name.trim(), (params, ret)))
    })
}
