#![recursion_limit = "128"]
extern crate proc_macro;
use proc_macro::TokenStream;

use near_bindgen_core::*;
use quote::quote;
use syn::{parse_macro_input, File, ItemImpl};

// For debugging.
#[proc_macro_attribute]
pub fn show_streams(attr: TokenStream, item: TokenStream) -> TokenStream {
    println!("attr: \"{}\"", attr.to_string());
    println!("item: \"{}\"", item.to_string());
    item
}

#[proc_macro_attribute]
pub fn near_bindgen(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input: ItemImpl = parse_macro_input!(item as ItemImpl);
    let binding = binding_file();
    let header = header_file();
    let generated_code = process_impl(&input);
    TokenStream::from(quote! {
        #input
        #binding
        #header
        #generated_code
    })
}

fn binding_file() -> File {
    let data = include_bytes!("../res/binding.rs");
    let data = std::str::from_utf8(data).unwrap();
    syn::parse_file(data).unwrap()
}

fn header_file() -> File {
    let data = include_bytes!("../res/header.rs");
    let data = std::str::from_utf8(data).unwrap();
    syn::parse_file(data).unwrap()
}
