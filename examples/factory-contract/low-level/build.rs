use cargo_near_build::{bon, camino};
use duct::cmd;

fn main() {
    let manifest_path = camino::Utf8PathBuf::from("../../status-message").join("Cargo.toml");
    // =================================
    // this workaround with calling `cargo update` before building sub-contract
    // is specifically for current demo contract in `near-sdk`, which has certain hardships with tracking `Cargo.lock`-s continuously for examples
    // this is not recommended for use in production sub-contracts,
    // as such contracts won't verify with respect to WASM reproducibility;
    cmd!("cargo", "update", "--manifest-path", manifest_path.as_str())
        .run()
        .expect("no `cargo update` err");
    // =================================
    let mut build_opts =
        cargo_near_build::BuildOpts::builder().manifest_path(manifest_path).build();
    // cargo-near caps the building rustc based on near-sdk's declared
    // `package.metadata.near.min_protocol_version`: < 84 (or unset) caps at 1.86
    // and rejects the bulk-memory opcodes rustc >= 1.87 emits. near-sdk now
    // declares PV 84, which lifts that cap, so this is no longer required; left
    // only to keep older near-sdk pins building.
    // The OptsBuilder doesn't expose this, so set the public field directly.
    build_opts.skip_rust_version_check = true;

    let extended_build_opts = cargo_near_build::extended::BuildOptsExtended::builder()
        .build_opts(build_opts)
        .rerun_if_changed_list(bon::vec!["Cargo.toml", "../Cargo.lock"])
        .result_file_path_env_key("BUILD_RS_SUB_BUILD_STATUS-MESSAGE")
        .prepare()
        .expect("no error in auto-compute of params");

    // the output `_wasm_path` can be reused in build.rs for more transformations, if needed
    //
    // required option `result_file_path_env_key` env variable is used in src/lib.rs to obtain the built wasm result:
    // const DONATION_DEFAULT_CONTRACT: &[u8] = include_bytes!(env!("BUILD_RS_SUB_BUILD_STATUS-MESSAGE1"));
    let _wasm_path =
        cargo_near_build::extended::build_with_cli(extended_build_opts).expect("no build error");
}
