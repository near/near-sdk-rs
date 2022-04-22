use proc_macro2::Ident;

pub fn generate_sim_proxy_struct(ident: &Ident) -> proc_macro2::TokenStream {
    use quote::{format_ident, quote};
    let name = format_ident!("{}Contract", ident);
    quote! {
      #[cfg(not(target_arch = "wasm32"))]
      pub struct #name {
        pub account_id: near_sdk::AccountId,
      }
    }
}
