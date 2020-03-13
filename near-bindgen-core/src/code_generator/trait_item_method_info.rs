use crate::info_extractor::{SerializerType, TraitItemMethodInfo};
use quote::quote;
use syn::export::TokenStream2;

impl TraitItemMethodInfo {
    /// Generate code that wraps the method.
    pub fn method_wrapper(&self) -> TokenStream2 {
        let ident = &self.attr_sig_info.ident;
        let ident_byte_str = &self.ident_byte_str;
        let pat_type_list = self.attr_sig_info.pat_type_list();
        let has_input_args = self.attr_sig_info.input_args().next().is_some();
        let struct_decl;
        let constructor;
        let value_ser;
        if !has_input_args {
            struct_decl = TokenStream2::new();
            constructor = TokenStream2::new();
            value_ser = quote! {let args = vec![]; };
        } else {
            struct_decl = self.attr_sig_info.input_struct();
            let constructor_call = self.attr_sig_info.constructor_expr();
            constructor = quote! {let args = #constructor_call;};
            value_ser = match self.attr_sig_info.result_serializer {
                SerializerType::JSON => quote! {
                    let args = serde_json::to_vec(&args).expect("Failed to serialize the cross contract args using JSON.");
                },
                SerializerType::Borsh => quote! {
                    let args = borsh::try_to_vec_with_schema(&args).expect("Failed to serialize the cross contract args using Borsh.");
                },
            };
        }
        quote! {
            pub fn #ident<T: ToString>(#pat_type_list __account_id: &T, __balance: near_bindgen::Balance, __gas: near_bindgen::Gas) -> near_bindgen::Promise {
                #struct_decl
                #constructor
                #value_ser
                near_bindgen::Promise::new(__account_id.to_string())
                .function_call(
                    #ident_byte_str.to_vec(),
                    args,
                    __balance,
                    __gas,
                )
            }
        }
    }
}
