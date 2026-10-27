use crate::core_impl::info_extractor::{AttrSigInfo, SerializerType};
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;

pub fn generate_serializer(
    attr_sig_info: &AttrSigInfo,
    serializer: &SerializerType,
) -> TokenStream2 {
    let krate = &attr_sig_info.krate;
    let has_input_args = attr_sig_info.input_args().next().is_some();
    if !has_input_args {
        return quote! { ::std::vec![] };
    }
    let struct_decl = attr_sig_info.input_struct_ser();
    let constructor_call = attr_sig_info.constructor_expr_ref();
    let constructor = quote! { let __args = #constructor_call; };
    let value_ser = match serializer {
        SerializerType::JSON => quote! {
            match #krate::serde_json::to_vec(&__args) {
                Ok(serialized) => serialized,
                Err(_) => #krate::env::panic_str("Failed to serialize the cross contract args using JSON."),
            }
        },
        SerializerType::Borsh => quote! {
            match #krate::borsh::to_vec(&__args) {
                Ok(serialized) => serialized,
                Err(_) => #krate::env::panic_str("Failed to serialize the cross contract args using Borsh."),
            }
        },
    };

    quote! {
        {
            #struct_decl
            #constructor
            #value_ser
        }
    }
}
