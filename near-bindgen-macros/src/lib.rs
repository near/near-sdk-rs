#![recursion_limit = "128"]
extern crate proc_macro;

use proc_macro::TokenStream;

use near_bindgen_core::*;
use near_bindgen_promise::process_trait;
use proc_macro2::Span;
use quote::quote;
use syn::export::TokenStream2;
use syn::{File, ItemImpl, ItemStruct, ItemTrait};

#[proc_macro_attribute]
pub fn near_bindgen(attr: TokenStream, item: TokenStream) -> TokenStream {
    if let Ok(input) = syn::parse::<ItemStruct>(item.clone()) {
        let sys_file = rust_file(include_bytes!("../res/sys.rs"));
        let near_environment = rust_file(include_bytes!("../res/near_blockchain.rs"));
        return TokenStream::from(quote! {
            #input
            #sys_file
            #near_environment
        });
    } else if let Ok(input) = syn::parse::<ItemImpl>(item) {
        let generated_code = process_impl(&input, TokenStream2::from(attr));
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

#[proc_macro_attribute]
pub fn ext_contract(attr: TokenStream, item: TokenStream) -> TokenStream {
    if let Ok(input) = syn::parse::<ItemTrait>(item.clone()) {
        let mut mod_name: Option<proc_macro2::Ident> = None;
        if !attr.is_empty() {
            mod_name = match syn::parse(attr) {
                Ok(x) => x,
                Err(err) => {
                    return TokenStream::from(
                        syn::Error::new(
                            Span::call_site(),
                            format!("Failed to parse mod name for ext_contract: {}", err),
                        )
                        .to_compile_error(),
                    )
                }
            };
        }
        match process_trait(&input, mod_name.map(|x| x.to_string())) {
            Ok(generated_code) => TokenStream::from(quote! {
                #input
                #generated_code
            }),
            Err(e) => TokenStream::from(e.to_compile_error()),
        }
    } else {
        TokenStream::from(
            syn::Error::new(Span::call_site(), "ext_contract can only be used on traits")
                .to_compile_error(),
        )
    }
}

/// `callback_args` is a marker attribute it does not generate code by itself.
#[proc_macro_attribute]
pub fn callback_args(_attr: TokenStream, item: TokenStream) -> TokenStream {
    item
}

/// `callback_args_vec` is a marker attribute it does not generate code by itself.
#[proc_macro_attribute]
pub fn callback_args_vec(_attr: TokenStream, item: TokenStream) -> TokenStream {
    item
}
