use crate::info_extractor::{InputStructType, SerializerType, TraitItemMethodInfo};
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
        let value_ser = if !has_input_args {
            struct_decl = TokenStream2::new();
            constructor = TokenStream2::new();
            quote! {let args = vec![]; }
        } else {
            struct_decl = self.attr_sig_info.input_struct(InputStructType::Serialization);
            let constructor_call = self.attr_sig_info.constructor_expr();
            constructor = quote! {let args = #constructor_call;};
            match self.attr_sig_info.result_serializer {
                SerializerType::JSON => quote! {
                    let args = near_sdk::serde_json::to_vec(&args).expect("Failed to serialize the cross contract args using JSON.");
                },
                SerializerType::Borsh => quote! {
                    let args = let result = near_sdk::borsh::BorshSerialize::try_to_vec(&args).expect("Failed to serialize the cross contract args using Borsh.");
                },
            }
        };
        quote! {
            pub fn #ident<T: ToString>(#pat_type_list __account_id: &T, __balance: near_sdk::Balance, __gas: near_sdk::Gas) -> near_sdk::Promise {
                #struct_decl
                #constructor
                #value_ser
                near_sdk::Promise::new(__account_id.to_string())
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
