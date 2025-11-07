use darling::FromMeta;
use proc_macro2::Ident;
use quote::quote;
use syn::{Expr, Generics};

#[derive(Default, FromMeta)]
#[darling(from_word = || Ok(Default::default()))]
pub struct ContractStateArgs {
    key: Option<Expr>,
}

impl quote::ToTokens for ContractStateArgs {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        if let Some(key) = &self.key {
            tokens.extend(quote! {key = #key});
        }
    }
}

impl ContractStateArgs {
    pub fn impl_contract_state(
        self,
        ident: &Ident,
        generics: &Generics,
    ) -> proc_macro2::TokenStream {
        let key = self.key.map(|key| {
            quote! {
                fn state_key() -> &'static [u8] {
                    #key
                }
            }
        });
        quote! {
            const _: () = {
                impl ::near_sdk::state::ContractState for #generics #ident #generics {
                    #key
                }
            };
        }
    }
}
