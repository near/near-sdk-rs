use super::ext::generate_ext_structs;
use quote::{format_ident, quote};
use syn::ItemStruct;

pub fn generate_sim_proxy_struct(input: &ItemStruct) -> proc_macro2::TokenStream {
    let ident = &input.ident;
    let new_name = format_ident!("{}Contract", ident);
    let name = quote! {#new_name};
    quote! {
         #[cfg(not(target_arch = "wasm32"))]
         pub struct #name {
            pub account_id: near_sdk::AccountId,
          }
    }
}

pub fn generate_ext_struct(input: &ItemStruct) -> proc_macro2::TokenStream {
    generate_ext_structs(&input.ident, Some(&input.generics))
}


