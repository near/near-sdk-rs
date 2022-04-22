use syn::ItemEnum;

pub fn generate_proxy_enum(input: &ItemEnum) -> proc_macro2::TokenStream {
    use quote::{format_ident, quote};
    let ident = &input.ident;
    let name = format_ident!("{}Contract", ident);
    quote! {
        #[cfg(not(target_arch = "wasm32"))]
        pub struct #name {
            pub account_id: near_sdk::AccountId,
        }
    }
}
