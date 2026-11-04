use proc_macro::TokenStream;
use proc_macro2::Span;

use quote::quote;

use darling::Error;
use darling::FromMeta;
use darling::ast::NestedMeta;
use syn::{ItemEnum, LitStr, parse_quote};

use crate::core_impl::utils::crate_path_string;

#[derive(Default, FromMeta, Clone, Debug)]
pub struct MacroConfig {
    pub event_json: Option<EventsConfig>,
    /// Path to the `near-sdk` crate to use in the generated code, forwarded from
    /// `#[near(event_json(...), crate = "...")]` / `#[near_bindgen(event_json(...), crate =
    /// "...")]`. Also forwarded into the `EventMetadata` derive via `#[event_metadata(crate =
    /// "...")]`, since that derive has no other way to learn the resolved crate path.
    #[darling(rename = "crate")]
    krate: Option<syn::Path>,
}
#[derive(Default, FromMeta, Clone, Debug)]
pub struct EventsConfig {
    standard: Option<String>,
}

/// this function is used to inject serialization macros and the `near_sdk::EventMetadata` macro.
/// In addition, this function extracts the event's `standard` value and injects it as a constant to be used by
/// the `near_sdk::EventMetadata` derive macro
pub(crate) fn near_events(attr: TokenStream, item: TokenStream) -> TokenStream {
    // get standard from attr args

    let attr_args = match NestedMeta::parse_meta_list(attr.into()) {
        Ok(v) => v,
        Err(e) => {
            return TokenStream::from(Error::from(e).write_errors());
        }
    };

    let args = match MacroConfig::from_list(&attr_args) {
        Ok(v) => v,
        Err(e) => {
            return TokenStream::from(e.write_errors());
        }
    };
    let near_sdk_crate: syn::Path = args.krate.clone().unwrap_or_else(|| parse_quote!(::near_sdk));
    let near_sdk_crate_str = crate_path_string(&near_sdk_crate);
    let serde_crate_str = format!("{near_sdk_crate_str}::serde");
    if let Some(standard) = args.event_json.and_then(|event_json| event_json.standard) {
        if let Ok(mut input) = syn::parse::<ItemEnum>(item) {
            let name = &input.ident;
            let standard_name = format!("{name}_event_standard");
            let standard_ident = syn::Ident::new(&standard_name, Span::call_site());
            // NearEvent Macro handles implementation
            input.attrs.push(
                parse_quote! (#[derive(#near_sdk_crate::serde::Serialize, #near_sdk_crate::EventMetadata)]),
            );
            input.attrs.push(parse_quote! (#[serde(crate = #serde_crate_str)]));
            input.attrs.push(parse_quote! (#[serde(tag = "event", content = "data")]));
            input.attrs.push(parse_quote! (#[serde(rename_all = "snake_case")]));
            // Forwarded so the `EventMetadata` derive (which can't otherwise see how it was
            // invoked) knows the resolved crate path too.
            input.attrs.push(parse_quote! (#[event_metadata(crate = #near_sdk_crate_str)]));

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
    } else {
        TokenStream::from(
            syn::Error::new(
                Span::call_site(),
                "Near events must have a `standard` value as an argument for `event_json` in the `near_bindgen` arguments. The value must be a string literal, e.g. \"nep999\", \"mintbase-marketplace\".",
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
