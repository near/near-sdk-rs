use crate::info_extractor::{SerializerType, TraitItemMethodInfo};
use quote::quote;
use syn::export::TokenStream2;

impl TraitItemMethodInfo {
    /// Generate code that wraps the method.
    pub fn method_wrapper(&self) -> TokenStream2 {
        let ident = &self.attr_sig_info.ident;
        let ident_byte_str = &self.ident_byte_str;
        let pat_type_list = self.attr_sig_info.pat_type_list();
        let args = self.attr_sig_info.arg_list();
        let struct_decl;
        let constructor;
        let value_ser;
        if args.is_empty() {
            struct_decl = TokenStream2::new();
            constructor = TokenStream2::new();
            value_ser = quote! {let args = vec![]; };
        } else {
            struct_decl = self.attr_sig_info.input_struct();
            let constructor_call = self.attr_sig_info.constructor_expr();
            constructor = quote! {let args = # constructor_call;};
            value_ser = match self.attr_sig_info.result_serializer {
                SerializerType::JSON => quote! {
                    let args = serde_json::to_vec(&args).expect("Failed to serialize the cross contract args using JSON.");
                },
                SerializerType::Borsh => quote! {
                    let args = borsh::BorshSerialize::try_to_vec(&args).expect("Failed to serialize the cross contract args using Borsh.");
                },
            };
        }
        quote! {
            pub fn #ident<T: ToString>(#pat_type_list __account_id: &T, __balance: Balance, __gas: Gas) -> Promise {
                #struct_decl
                #constructor
                #value_ser
                Promise::new(__account_id.to_string())
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
