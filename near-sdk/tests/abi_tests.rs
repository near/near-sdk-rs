use std::{collections::HashSet, env, fs, path::Path, process::Command};

// These methods are prepended to the contract internally, update this test list if they change
const PREPENDED_METHODS: [&str; 1] = ["contract_source_metadata"];

#[test]
fn ensure_abi_for_prepended_functions() {
    const NEAR_SDK_DIR: &str = env!("CARGO_MANIFEST_DIR");

    // using the adder example as a test case
    let target = Path::new(NEAR_SDK_DIR).join("../examples/adder/target");
    let project_manifest = Path::new(NEAR_SDK_DIR).join("../examples/adder/Cargo.toml");

    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    let res = Command::new(cargo)
        .arg("build")
        .args(["--manifest-path", &project_manifest.to_string_lossy()])
        .args(["--features", "near-sdk/__abi-generate"])
        .env("CARGO_TARGET_DIR", &target)
        .env("RUSTFLAGS", "-Awarnings")
        .output()
        .unwrap();

    assert!(
        res.status.success(),
        "failed to compile contract abi: {}",
        String::from_utf8_lossy(&res.stderr)
    );

    let dylib_file = target.join(format!("debug/libadder.{}", dylib_extension()));
    assert!(dylib_file.exists(), "Build file should exist");

    let dylib_file_contents = fs::read(dylib_file).expect("unable to read build file");

    let near_abi_symbols = symbolic_debuginfo::Object::parse(&dylib_file_contents)
        .expect("unable to parse dylib")
        .symbols()
        .flat_map(|sym| sym.name)
        .filter(|sym_name| sym_name.starts_with("__near_abi_"))
        .collect::<HashSet<_>>();

    // ensure methods are prepended
    PREPENDED_METHODS.map(|method| {
        assert!(
            near_abi_symbols.contains(format!("__near_abi_{}", method).as_str()),
            "ABI should contain prepended method {}",
            method
        );
    });
}

const fn dylib_extension() -> &'static str {
    #[cfg(target_os = "linux")]
    return "so";

    #[cfg(target_os = "macos")]
    return "dylib";

    #[cfg(target_os = "windows")]
    return "dll";

    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    compile_error!("Unsupported platform");
}
