use proc_macro2::TokenStream as TokenStream2;
use quote::quote;

pub fn embed() -> TokenStream2 {
    // `CARGO_NEAR_ABI_PATH` must never become a compile-time dependency of near-sdk-macros
    // itself (neither via `env!` here nor via `option_env!`/`rerun-if-env-changed` in a build
    // script): the path is contract-specific, so fingerprinting this crate on it recompiles
    // near-sdk-macros — and every crate depending on it — each time the path changes, e.g. on
    // every contract switch in a multi-contract workspace, or on every build in repos that
    // pass a freshly `mktemp`-ed `--out-dir` to cargo-near.
    //
    // Instead, `env!`/`option_env!` are emitted into the generated code, attaching the env
    // (and file) dependency to the contract crate being expanded, which has a single stable
    // value for both. The `std::env` read below only selects the expansion branch: it is
    // deliberately untracked, and each branch tracks the variable in the emitted tokens.
    if std::env::var_os("CARGO_NEAR_ABI_PATH").is_some() {
        quote! {
            const _: () = {
                const __CONTRACT_ABI: &'static [u8] =
                    ::std::include_bytes!(::std::env!("CARGO_NEAR_ABI_PATH"));
                #[unsafe(no_mangle)]
                pub extern "C" fn __contract_abi() {
                    ::near_sdk::env::value_return(__CONTRACT_ABI);
                }
            };
        }
    } else {
        // The `__abi-embed` feature is private and only expected to be activated by
        // https://github.com/near/cargo-near, which always sets `CARGO_NEAR_ABI_PATH`.
        // Activated manually without the env var, embedding is skipped; the `option_env!`
        // below keeps the contract crate's fingerprint tracking the variable, so a later
        // build that does provide it recompiles the contract and embeds the ABI.
        quote! {
            const _: ::core::option::Option<&'static str> =
                ::core::option_env!("CARGO_NEAR_ABI_PATH");
        }
    }
}
