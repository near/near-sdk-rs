use proc_macro2::TokenStream as TokenStream2;
use quote::quote;

pub fn embed() -> TokenStream2 {
    let abi_path = env!("CARGO_NEAR_ABI_PATH");
    quote! {
        const _: () = {
            const __CONTRACT_ABI: &'static [u8] = ::std::include_bytes!(#abi_path);
            #[no_mangle]
            pub extern "C" fn __contract_abi() {
                ::near_sdk::env::value_return(__CONTRACT_ABI);
            }
        };
    }
}
