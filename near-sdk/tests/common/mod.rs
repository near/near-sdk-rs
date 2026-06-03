//! Shared helpers for integration tests that need a sandbox-deployable wasm
//! artifact for `tests/test-contracts/*`.
//!
//! Builds the wasm directly with `cargo build` instead of going through
//! `near-workspaces::compile_project` / `cargo near build`.
//!
//! Historical context: this bypass was added during the 2.12-RC bump because
//! `cargo near build` (the path `compile_project` takes) capped the building
//! rustc based on near-sdk's declared `package.metadata.near.min_protocol_version`
//! — when that was < 84 (or unset) it rejected the bulk-memory opcodes rustc
//! >= 1.87 emits, and `compile_project` did not expose `skip_rust_version_check`.
//! A plain `cargo build --target wasm32-unknown-unknown --release` produces the
//! same `.wasm` we ultimately deploy — we just lose the ABI embedding step,
//! which none of the sandbox tests in this directory care about.
//!
//! Both halves of that are now resolved: near-sdk declares PV 84 (no rustc cap),
//! and near-workspaces 0.22.2 `compile_project` passes `skip_rust_version_check`
//! internally anyway — so this helper can be replaced with
//! `near_workspaces::compile_project(...)`.

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
