use proc_macro2::TokenStream as TokenStream2;
use quote::quote;

pub fn embed() -> TokenStream2 {
    // `env!`/`option_env!` are emitted into the generated code rather than evaluated here:
    // reading `CARGO_NEAR_ABI_PATH` while compiling near-sdk-macros itself would fingerprint
    // this crate — and every crate above it — on a contract-specific path, rebuilding the
    // whole graph whenever it changes. The untracked read below only selects the branch;
    // each branch tracks the variable in the emitted tokens.
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
        // Feature activated without cargo-near setting the env var: skip embedding, but keep
        // the contract crate tracking the variable so a later build that provides it re-embeds.
        quote! {
            const _: ::core::option::Option<&'static str> =
                ::core::option_env!("CARGO_NEAR_ABI_PATH");
        }
    }
}
