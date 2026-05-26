//! Shared helpers for integration tests that need a sandbox-deployable wasm
//! artifact for `tests/test-contracts/*`.
//!
//! Builds the wasm directly with `cargo build` instead of going through
//! `near-workspaces::compile_project` / `cargo near build`. This bypasses two
//! problems that block sandbox-backed integration tests after the 2.12-RC bump:
//!
//! 1. `cargo near build` (the path `compile_project` takes) hard-refuses to
//!    build contracts with rustc >= 1.87 unless `--skip-rust-version-check` is
//!    passed, and the `compile_project` API does not expose that knob.
//! 2. A plain `cargo build --target wasm32-unknown-unknown --release` produces
//!    the same `.wasm` we ultimately deploy — we just lose the ABI embedding
//!    step, which none of the sandbox tests in this directory care about.
//!
//! Once `near-workspaces` exposes a `skip_rust_version_check` option (or
//! `cargo-near` itself relaxes the check for >=1.93), this helper can be
//! replaced with `near_workspaces::compile_project(...)` again.

#![allow(dead_code)]

use std::path::PathBuf;

/// Builds the wasm artifact for `tests/test-contracts/<name>/` and returns its bytes.
pub fn build_test_contract(name: &str) -> anyhow::Result<Vec<u8>> {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let project_dir: PathBuf = [manifest_dir, "tests", "test-contracts", name].iter().collect();
    let manifest_path = project_dir.join("Cargo.toml");

    let status = std::process::Command::new(env!("CARGO"))
        .args(["build", "--release", "--target", "wasm32-unknown-unknown", "--manifest-path"])
        .arg(&manifest_path)
        // Wipe inherited cargo flags that would otherwise be propagated from the
        // parent invocation (in particular `--target` and any flags that imply
        // host artifacts) — `cargo build` in a wasm context needs a clean slate.
        .env_remove("CARGO_BUILD_TARGET")
        .env_remove("CARGO_ENCODED_RUSTFLAGS")
        .env_remove("RUSTFLAGS")
        .status()?;
    if !status.success() {
        anyhow::bail!("cargo build for test-contract `{name}` failed with {status}");
    }

    let wasm_path = project_dir
        .join("target")
        .join("wasm32-unknown-unknown")
        .join("release")
        .join(format!("{}.wasm", name.replace('-', "_")));
    let wasm = std::fs::read(&wasm_path)
        .map_err(|e| anyhow::anyhow!("failed to read wasm at {}: {e}", wasm_path.display()))?;
    Ok(wasm)
}
