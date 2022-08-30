use proc_macro2::TokenStream as TokenStream2;
use quote::quote;

pub fn embed() -> TokenStream2 {
    let abi_path = match option_env!("CARGO_NEAR_ABI_PATH") {
        Some(path) => path,
        None => {
            return quote! {
                compile_error!(
                    "the `__abi-embed` feature flag is private and should not be activated manually\n\
                    \n\
                    help\x1b[0m: consider using https://github.com/near/cargo-near"
                );
            };
        }
    };
    quote! {
        const _: () = {
            const __CONTRACT_ABI: &'static [u8] = include_bytes!(#abi_path);
            #[no_mangle]
            pub extern "C" fn __contract_abi() {
                near_sdk::env::value_return(__CONTRACT_ABI);
            }
        };
    }
}
