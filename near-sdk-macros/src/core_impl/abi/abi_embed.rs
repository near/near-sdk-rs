use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::quote;

pub fn embed() -> TokenStream2 {
    let abi_path = match option_env!("CARGO_NEAR_ABI_PATH") {
        Some(path) => path,
        None => {
            return syn::Error::new(
                Span::call_site(),
                "CARGO_NEAR_ABI_PATH environment variable is not set",
            )
            .to_compile_error()
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
