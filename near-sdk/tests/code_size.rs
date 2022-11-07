/// Compiles contract to wasm with release configuration and returns the code size.
fn check_example_size(example: &str) -> usize {
    let status = std::process::Command::new("cargo")
        .env("RUSTFLAGS", "-C link-arg=-s")
        .args(["build", "--release", "--target", "wasm32-unknown-unknown", "--manifest-path"])
        .arg(format!("../examples/{}/Cargo.toml", example))
        .status()
        .unwrap();
    if !status.success() {
        panic!("building wasm example returned non-zero code {}", status);
    }

    let wasm = std::fs::read(format!(
        "../examples/{}/target/wasm32-unknown-unknown/release/{}.wasm",
        example,
        example.replace('-', "_")
    ))
    .unwrap();

    wasm.len()
}

#[test]
fn lock_fungible_code_size_check() {
    let size = check_example_size("lockable-fungible-token");

    // Current contract size at the time of writing this test is 141_474, giving about ~10% buffer.
    assert!(size < 155_000);
}

#[test]
fn status_message_code_size_check() {
    let size = check_example_size("status-message");

    // Currently 123821.
    assert!(size < 135_000);
}
