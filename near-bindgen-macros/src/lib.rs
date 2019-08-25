#![recursion_limit = "128"]
extern crate proc_macro;

use proc_macro::TokenStream;

use near_bindgen_core::*;
use proc_macro2::Span;
use quote::quote;
use syn::{File, ItemImpl, ItemStruct};

#[proc_macro_attribute]
pub fn near_bindgen(_attr: TokenStream, item: TokenStream) -> TokenStream {
    if let Ok(input) = syn::parse::<ItemStruct>(item.clone()) {
        let sys_file = rust_file(include_bytes!("../res/sys.rs"));
        let near_environment = rust_file(include_bytes!("../res/near_blockchain.rs"));
        return TokenStream::from(quote! {
            #input
            #sys_file
            #near_environment
        });
    } else if let Ok(input) = syn::parse::<ItemImpl>(item) {
        let generated_code = process_impl(&input);
        TokenStream::from(quote! {
            #input
            #generated_code
        })
    } else {
        TokenStream::from(
            syn::Error::new(
                Span::call_site(),
                "near_bindgen can only be used on type declarations and impl sections.",
            )
            .to_compile_error(),
        )
    }
}

fn rust_file(data: &[u8]) -> File {
    let data = std::str::from_utf8(data).unwrap();
    syn::parse_file(data).unwrap()
}
