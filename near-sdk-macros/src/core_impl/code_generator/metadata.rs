use proc_macro2::Ident;
use quote::quote;
use syn::Generics;

/// Generates a view method to retrieve the source metadata.
pub(crate) fn generate_contract_metadata_method(
    ident: &Ident,
    generics: &Generics,
    near_sdk_crate: &syn::Path,
) -> proc_macro2::TokenStream {
    quote! {
        impl #generics #ident #generics {
            pub fn contract_source_metadata() {
                #near_sdk_crate::env::value_return(CONTRACT_SOURCE_METADATA.as_bytes())
            }
        }
    }
}
