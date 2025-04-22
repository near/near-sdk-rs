pub fn is_abi_build_step_or_debug_profile() -> bool {
    if let Ok(value) = std::env::var("CARGO_NEAR_ABI_GENERATION") {
        if value == "true" {
            return true;
        }
    }
    if let Ok(value) = std::env::var("PROFILE") {
        if value == "debug" {
            return true;
        }
    }
    false
}

fn override_cargo_target_dir() -> cargo_near_build::camino::Utf8PathBuf {
    let out_dir_env = std::env::var("OUT_DIR").expect("OUT_DIR is always set in build scripts");
    let out_dir = cargo_near_build::camino::Utf8PathBuf::from(out_dir_env);

    let dir =
        out_dir.join(format!("target-{}-for-{}", "status-message", "factory-contract-high-level"));

    std::fs::create_dir_all(&dir).expect("create dir");
    dir
}
fn main() {
    // directory of target `status-message` sub-contract's crate
    let workdir = "../../status-message".to_string();
    let manifest = cargo_near_build::camino::Utf8PathBuf::from(workdir.clone()).join("Cargo.toml");
    // unix path to target `status-message` sub-contract's crate from root of the repo
    let nep330_contract_path = "examples/status-message";

    // this is wasm result path of `cargo near build non-reproducible-wasm`
    // for target sub-contract, where repo's root is replaced with `/home/near/code`
    let nep330_output_wasm_path: &str =
        "/home/near/code/examples/status-message/target/near/status_message.wasm";

    let override_cargo_target_dir = override_cargo_target_dir();
    let build_opts = cargo_near_build::BuildOpts::builder()
        .manifest_path(manifest)
        .override_cargo_target_dir(override_cargo_target_dir.to_string())
        .override_nep330_contract_path(nep330_contract_path)
        .override_nep330_output_wasm_path(nep330_output_wasm_path)
        .build();

    let is_abi_or_cargo_check = is_abi_build_step_or_debug_profile();

    let out_path = run_build::step(build_opts, is_abi_or_cargo_check, &override_cargo_target_dir);

    post_build::step(
        is_abi_or_cargo_check,
        vec![&workdir, "Cargo.toml", "../Cargo.lock"],
        out_path,
        "BUILD_RS_SUB_BUILD_ARTIFACT_1",
    );
}

mod run_build {

    pub fn step(
        opts: cargo_near_build::BuildOpts,
        is_abi_or_cargo_check: bool,
        override_cargo_target_dir: &cargo_near_build::camino::Utf8PathBuf,
    ) -> cargo_near_build::camino::Utf8PathBuf {
        if is_abi_or_cargo_check {
            let out_path = override_cargo_target_dir.join("empty_subcontract_stub.wasm");
            std::fs::write(&out_path, b"").expect("success write");
            out_path
        } else {
            cargo_near_build::build_with_cli(opts).expect("successfull build")
        }
    }
}

mod post_build {

    use cargo_near_build::camino::Utf8PathBuf;
    pub fn step(
        is_abi_or_cargo_check: bool,
        watched_paths: Vec<&str>,
        out_path: Utf8PathBuf,
        result_env_var: &str,
    ) {
        for in_path in watched_paths {
            println!("cargo::rerun-if-changed={}", in_path);
        }
        println!("cargo::rustc-env={}={}", result_env_var, out_path.as_str());
        if is_abi_or_cargo_check {
            println!(
                "cargo::warning={}",
                format!("subcontract empty stub is `{}`", out_path.as_str())
            );
        } else {
            println!(
                "cargo::warning={}",
                format!("subcontract out path is `{}`", out_path.as_str())
            );
        }

        if !is_abi_or_cargo_check {
            println!(
                "cargo::warning={}",
                format!(
                    "subcontract sha256 is {}",
                    cargo_near_build::SHA256Checksum::new(&out_path)
                        .expect("read file")
                        .to_base58_string()
                )
            );
        }
    }
}
