use darling::FromMeta;
use proc_macro2::Ident;
use quote::quote;
use syn::{parse_quote, Expr, Generics};

#[derive(Default, FromMeta)]
#[darling(from_word = || Ok(Default::default()))]
pub(crate) struct ContractState {
    key: Option<Expr>,
}

impl quote::ToTokens for ContractState {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        if let Some(key) = &self.key {
            tokens.extend(quote! {key = #key});
        }
    }
}

impl ContractState {
    pub fn generate_root_state_access_methods(
        self,
        ident: &Ident,
        generics: &Generics,
    ) -> proc_macro2::TokenStream {
        let key = self.key.unwrap_or(parse_quote!(::near_sdk::env::DEFAULT_STATE_KEY));
        quote! {
            const _: () = {
                impl #generics #ident #generics {
                    #[inline]
                    pub(crate) fn __state_key() -> &'static [u8] {
                        #key
                    }
                    #[inline]
                    pub(crate) fn __state_exists() -> bool {
                        ::near_sdk::env::storage_has_key(Self::__state_key())
                    }
                    #[inline]
                    #[track_caller]
                    pub(crate) fn __state_read() -> Option<Self> {
                        ::near_sdk::env::storage_read(Self::__state_key()).map(|data| {
                            ::near_sdk::borsh::from_slice(&data)
                                .unwrap_or_else(|_| ::near_sdk::env::panic_str("Cannot deserialize the contract state."))
                        })
                    }
                    #[inline]
                    #[track_caller]
                    pub(crate) fn __state_write(self) -> bool {
                        let data = ::near_sdk::borsh::to_vec(&self)
                            .unwrap_or_else(|_| ::near_sdk::env::panic_str("Cannot serialize the contract state."));
                        ::near_sdk::env::storage_write(Self::__state_key(), &data)
                    }
                }
            };
        }
    }
}
