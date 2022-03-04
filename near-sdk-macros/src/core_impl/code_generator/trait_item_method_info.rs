use crate::core_impl::{info_extractor::TraitItemMethodInfo, serializer};
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;

impl TraitItemMethodInfo {
    /// Generate code that wraps the method.
    pub fn method_wrapper(&self) -> TokenStream2 {
        let ident = &self.attr_sig_info.ident;
        let ident_byte_str = &self.ident_byte_str;
        let pat_type_list = self.attr_sig_info.pat_type_list();
        let serialize = serializer::generate_serializer(
            &self.attr_sig_info,
            &self.attr_sig_info.result_serializer,
        );
        quote! {
            pub fn #ident(#pat_type_list __account_id: AccountId, __balance: near_sdk::Balance, __gas: near_sdk::Gas) -> near_sdk::Promise {
                #serialize
                near_sdk::Promise::new(__account_id)
                .function_call(
                    #ident_byte_str.to_string(),
                    args,
                    __balance,
                    __gas,
                )
            }
        }
    }
}
