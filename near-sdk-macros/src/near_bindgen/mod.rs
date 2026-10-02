use proc_macro2::TokenStream;
use quote::{quote, format_ident};
use syn::{parse_macro_input, ItemStruct, AttributeArgs, Ident};

use crate::utils::parse_crate_name;

pub fn expand(attr: TokenStream, item: TokenStream) -> TokenStream {
    let args = parse_macro_input!(attr as AttributeArgs);
    let crate_name = parse_crate_name(&args);
    let crate_ident = Ident::new(&crate_name, proc_macro2::Span::call_site());

    let mut item = parse_macro_input!(item as ItemStruct);

    // Preserve original struct attributes and visibility
    let struct_name = &item.ident;
    let generics = &item.generics;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    // Build the expanded code using the configurable crate path instead of hard-coded `near_sdk`
    let expanded = quote! {
        #item

        impl #impl_generics #crate_ident::AccountId {
            // placeholder for generated bindings
        }

        impl #impl_generics #crate_ident:: BorshSerialize for #struct_name #ty_generics #where_clause {
            fn serialize<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()> {
                unimplemented!()
            }
        }

        impl #impl_generics #crate_ident::BorshDeserialize for #struct_name #ty_generics #where_clause {
            fn deserialize_reader<R: std::io::Read>(reader: &mut R) -> std::io::Result<Self> {
                unimplemented!()
            }
        }
    };

    expanded.into()
}
