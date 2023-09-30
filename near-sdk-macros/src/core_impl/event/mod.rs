use proc_macro::TokenStream;
use proc_macro2::Span;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::meta::ParseNestedMeta;
use syn::parenthesized;
use syn::parse_macro_input;
use syn::Attribute;
use syn::Meta;
use syn::{parse_quote, DeriveInput, ItemEnum, LitStr, MetaNameValue};
/// this function is used to inject serialization macros and the `near_sdk::EventMetadata` macro.
/// In addition, this function extracts the event's `standard` value and injects it as a constant to be used by
/// the `near_sdk::EventMetadata` derive macro
pub(crate) fn near_events(attr: TokenStream, item: TokenStream) -> TokenStream {
    // get standard from attr args
    let args = parse_macro_input!(attr as syn::Meta);
    let standard = get_standard_arg(args);
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
        input.attrs.push(
            parse_quote! (#[derive(::near_sdk::serde::Serialize, ::near_sdk::EventMetadata)]),
        );
        input.attrs.push(parse_quote! (#[serde(crate="::near_sdk::serde")]));
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
        if attr.path().is_ident("event_version") {
            return attr.parse_args::<LitStr>().ok();
        }
    }
    None
}

/// this function returns the `standard` value from `#[near_bindgen(event_json(standard = "nepXXX"))]`
fn get_standard_arg(meta: syn::Meta) -> Option<LitStr> {
    let mut standard: Option<LitStr> = None;
    if meta.path().is_ident("event_json") {
        match meta {
            Meta::NameValue(named_value) => {
                if let MetaNameValue { path, value, .. } = named_value {
                    if path.is_ident("standard") {
                        match value {
                            syn::Expr::Lit(lit_str) => {
                                if let syn::Lit::Str(lit_str) = lit_str.lit {
                                    standard = Some(lit_str);
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
            Meta::Path(path) => {
                if path.is_ident("standard") {
                    standard = Some(LitStr::new("nep141", Span::call_site()));
                }
            }
            Meta::List(list) => {
                let _ = list.parse_nested_meta(|meta| {
                    if meta.path.is_ident("standard") {
                        let value = meta.value()?; // this parses the `=`
                        let s: LitStr = value.parse()?; // this parses `"EarlGrey"`
                        standard = Some(s);
                    }

                    Ok(())
                });
            }
        }
    }
    standard
}
