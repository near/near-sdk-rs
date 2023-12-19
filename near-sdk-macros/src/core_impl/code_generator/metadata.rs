use proc_macro2::Ident;
use quote::quote;
use syn::Generics;

/// Generates a view method to retrieve the source metadata.
pub(crate) fn generate_contract_metadata_method(
    ident: &Ident,
    generics: &Generics,
) -> proc_macro2::TokenStream {
    quote! {
        impl #generics #ident #generics {
            pub fn contract_source_metadata() {
                near_sdk::env::value_return(CONTRACT_SOURCE_METADATA.as_bytes())
            }
        }
    }
}
