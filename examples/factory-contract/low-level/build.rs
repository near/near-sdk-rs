use cargo_near_build::{bon, camino};

fn main() {
    let build_opts = cargo_near_build::BuildOpts::builder()
        .manifest_path(camino::Utf8PathBuf::from("../../status-message").join("Cargo.toml"))
        // `--no-locked` flag isn't recommended to be used in build-scripts for
        // production contracts for reproducible builds,
        // as such contracts won't verify with respect to WASM reproducibility;
        // it's fine for current demo contract in `near-sdk`, which has certain hardships with tracking `Cargo.lock`-s continuously for examples
        .no_locked(true)
        .build();

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
