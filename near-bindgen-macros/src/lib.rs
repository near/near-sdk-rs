#![recursion_limit = "128"]
extern crate proc_macro;

use proc_macro::TokenStream;

use near_bindgen_core::*;
use quote::quote;
use syn::{parse_macro_input, ItemImpl};

#[proc_macro_attribute]
pub fn near_bindgen(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input: ItemImpl = parse_macro_input!(item as ItemImpl);
    let generated_code = process_impl(&input);
    TokenStream::from(quote! {
        #input
        #generated_code
    })
}
