//! Shared helpers for integration tests that need a sandbox-deployable wasm
//! artifact for `tests/test-contracts/*`.
//!
//! Builds the wasm through `near_workspaces::compile_project`, which shells out
//! to `cargo near build`. near-workspaces >= 0.22.4 (the dev-dependency floor)
//! makes `compile_project` pass `skip_rust_version_check` internally, and near-sdk
//! declares `package.metadata.near.min_protocol_version = 84` (which lifts
//! cargo-near's rustc cap), so building these contracts with current toolchains
//! works without any bypass.

#![allow(dead_code)]

/// Builds the wasm artifact for `tests/test-contracts/<name>/` and returns its bytes.
pub async fn build_test_contract(name: &str) -> anyhow::Result<Vec<u8>> {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let project_dir = format!("{manifest_dir}/tests/test-contracts/{name}");
    near_workspaces::compile_project(&project_dir)
        .await
        .map_err(|e| anyhow::anyhow!("compiling test-contract `{name}`: {e}"))
}
