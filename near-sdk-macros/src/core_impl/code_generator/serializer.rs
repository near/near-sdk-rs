use crate::core_impl::info_extractor::{AttrSigInfo, SerializerType};
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;

pub fn generate_serializer(
    attr_sig_info: &AttrSigInfo,
    serializer: &SerializerType,
) -> TokenStream2 {
    let has_input_args = attr_sig_info.input_args().next().is_some();
    if !has_input_args {
        return quote! { ::std::vec![] };
    }
    let struct_decl = attr_sig_info.input_struct_ser();
    let constructor_call = attr_sig_info.constructor_expr_ref();
    let constructor = quote! { let __args = #constructor_call; };
    let value_ser = match serializer {
        SerializerType::JSON => quote! {
            match near_sdk::serde_json::to_vec(&__args) {
                Ok(serialized) => serialized,
                Err(_) => ::near_sdk::env::panic_err(::near_sdk::errors::JsonSerializeError::new("cross contract args")),
            }
        },
        SerializerType::Borsh => quote! {
            match near_sdk::borsh::to_vec(&__args) {
                Ok(serialized) => serialized,
                Err(_) => ::near_sdk::env::panic_err(::near_sdk::errors::BorshSerializeError::new("cross contract args")),
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
