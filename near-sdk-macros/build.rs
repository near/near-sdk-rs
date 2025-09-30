fn main() {
    println!("cargo:rustc-check-cfg=cfg(feature, values(\"__abi-embed-checked\"))");
    if cfg!(feature = "__abi-embed") {
        if option_env!("CARGO_NEAR_ABI_PATH").is_some() {
            println!("cargo:rustc-cfg=feature=\"__abi-embed-checked\"");
        } else {
            println!("cargo:warning=the `__abi-embed` feature flag is private and should not be activated manually, ignoring");
            println!("cargo:warning=\x1b[1mhelp\x1b[0m: consider using https://github.com/near/cargo-near");
        }
    }
}
