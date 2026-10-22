use darling::FromMeta;
use proc_macro2::Ident;
use quote::quote;
use syn::{Expr, Generics};

#[derive(Default, FromMeta)]
#[darling(from_word = || Ok(Default::default()))]
pub struct ContractStateArgs {
    key: Option<Expr>,
    /// `layout = v2` selects the storage v2 (path-derived, tagged record) root.
    layout: Option<Ident>,
    /// Root byte for `layout = v2`; defaults to `0xF0`.
    root: Option<Expr>,
}

impl quote::ToTokens for ContractStateArgs {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let mut parts = Vec::new();
        if let Some(key) = &self.key {
            parts.push(quote! {key = #key});
        }
        if let Some(layout) = &self.layout {
            parts.push(quote! {layout = #layout});
        }
        if let Some(root) = &self.root {
            parts.push(quote! {root = #root});
        }
        tokens.extend(quote! {#(#parts),*});
    }
}

impl ContractStateArgs {
    /// `Some(root byte)` if this is a storage v2 root.
    pub fn v2_root(&self) -> Option<proc_macro2::TokenStream> {
        match &self.layout {
            Some(l) if l == "v2" => {
                Some(self.root.as_ref().map_or_else(|| quote! {0xF0u8}, |r| quote! {#r}))
            }
            _ => None,
        }
    }

    pub fn layout_error(&self) -> Option<syn::Error> {
        match (&self.layout, &self.root) {
            (Some(l), _) if l != "v2" => {
                Some(syn::Error::new(l.span(), "unknown storage layout; expected `layout = v2`"))
            }
            (None, Some(_)) => Some(syn::Error::new(
                proc_macro2::Span::call_site(),
                "`root` requires `layout = v2`",
            )),
            _ => None,
        }
    }

    pub fn impl_contract_state(
        self,
        ident: &Ident,
        generics: &Generics,
    ) -> proc_macro2::TokenStream {
        if let Some(root) = self.v2_root() {
            return super::storage_v2::contract_state_v2(ident, &root);
        }
        let key = self.key.map(|key| {
            quote! {
                #[inline]
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
