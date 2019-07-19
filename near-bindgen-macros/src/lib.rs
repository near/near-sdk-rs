#![recursion_limit = "128"]
extern crate proc_macro;

use proc_macro::TokenStream;

use near_bindgen_core::*;
use quote::quote;
use syn::{parse_macro_input, File, ItemImpl};

#[proc_macro_attribute]
pub fn near_bindgen(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input: ItemImpl = parse_macro_input!(item as ItemImpl);
    let generated_code = process_impl(&input);
    let sys_file = rust_file(include_bytes!("../res/sys.rs"));
    let near_context = rust_file(include_bytes!("../res/near_context.rs"));
    TokenStream::from(quote! {
        #input
        #generated_code
        #sys_file
        #near_context
    })
}

fn rust_file(data: &[u8]) -> File {
    let data = std::str::from_utf8(data).unwrap();
    syn::parse_file(data).unwrap()
}
