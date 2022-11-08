use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn::{parse_quote, ItemEnum, LitStr};

/// this function is used to inject serialization macros and the `near_sdk::EventMetadata` macro.
/// In addition, this function extracts the event's `standard` value and injects it as a constant to be used by
/// the `near_sdk::EventMetadata` derive macro
pub(crate) fn near_events(attr: TokenStream, item: TokenStream) -> TokenStream {
    // get standard from attr args
    let standard = get_standard_arg(&syn::parse_macro_input!(attr as syn::AttributeArgs));
    if standard.is_none() {
        return TokenStream::from(
            syn::Error::new(
                Span::call_site(),
                "Near events must have a `standard` value as an argument for `event_json` in the `near_bindgen` arguments. The value must be a string literal, e.g. \"nep999\", \"mintbase-marketplace\".",
            )
            .to_compile_error(),
        );
    }

    if let Ok(mut input) = syn::parse::<ItemEnum>(item) {
        let name = &input.ident;
        let standard_name = format!("{}_event_standard", name);
        let standard_ident = syn::Ident::new(&standard_name, Span::call_site());
        // NearEvent Macro handles implementation
        input
            .attrs
            .push(parse_quote! (#[derive(near_sdk::serde::Serialize, near_sdk::EventMetadata)]));
        input.attrs.push(parse_quote! (#[serde(crate="near_sdk::serde")]));
        input.attrs.push(parse_quote! (#[serde(tag = "event", content = "data")]));
        input.attrs.push(parse_quote! (#[serde(rename_all = "snake_case")]));

        TokenStream::from(quote! {
            const #standard_ident: &'static str = #standard;
            #input
        })
    } else {
        TokenStream::from(
            syn::Error::new(
                Span::call_site(),
                "`#[near_bindgen(event_json(standard = \"nepXXX\"))]` can only be used as an attribute on enums.",
            )
            .to_compile_error(),
        )
    }
}

/// This function returns the `version` value from `#[event_version("x.x.x")]`.
/// used by `near_sdk::EventMetadata`
pub(crate) fn get_event_version(var: &syn::Variant) -> Option<LitStr> {
    for attr in var.attrs.iter() {
        if attr.path.is_ident("event_version") {
            return attr.parse_args::<LitStr>().ok();
        }
    }
    None
}

/// this function returns the `standard` value from `#[near_bindgen(event_json(standard = "nepXXX"))]`
fn get_standard_arg(args: &[syn::NestedMeta]) -> Option<LitStr> {
    let mut standard: Option<LitStr> = None;
    for arg in args.iter() {
        if let syn::NestedMeta::Meta(syn::Meta::List(syn::MetaList { path, nested, .. })) = arg {
            if path.is_ident("event_json") {
                for event_arg in nested.iter() {
                    if let syn::NestedMeta::Meta(syn::Meta::NameValue(syn::MetaNameValue {
                        path,
                        lit: syn::Lit::Str(value),
                        ..
                    })) = event_arg
                    {
                        if path.is_ident("standard") {
                            standard = Some(value.to_owned());
                            break;
                        }
                    }
                }
            }
        }
    }
    standard
}
