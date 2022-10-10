use proc_macro::TokenStream;

use proc_macro2::Span;
use quote::quote;
use syn::{parse_quote, ItemEnum};

/// This attribute macro is used on a enum and its implementations
/// to generate the necessary code to format standard event logs.
///
/// This macro will generate code to load and deserialize state if the `self` parameter is included
/// as well as saving it back to state if `&mut self` is used.
///
/// For parameter serialization, this macro will generate a struct with all of the parameters as
/// fields and derive deserialization for it. By default this will be JSON deserialized with `serde`
/// but can be overwritten by using `#[serializer(borsh)]`.
///
/// `#[near_bindgen]` will also handle serializing and setting the return value of the
/// function execution based on what type is returned by the function. By default, this will be
/// done through `serde` serialized as JSON, but this can be overwritten using
/// `#[result_serializer(borsh)]`.
///
/// # Examples
///
/// ```ignore
/// use near_sdk::{near_bindgen, Event};
///
/// #[near_bindgen(events)]
/// pub enum MyEvents {
///    #[event_standard("swap_standard")]
///    #[event_version("1.0.0")]
///    Swap { token_in: AccountId, token_out: AccountId, amount_in: u128, amount_out: u128 },
///
///    #[event_standard("string_standard")]
///    #[event_version("2.0.0")]
///    StringEvent(String),
///
///    #[event_standard("empty_standard")]
///    #[event_version("3.0.0")]
///    EmptyEvent
/// }
///
/// #[near_bindgen]
/// impl Contract {
///     pub fn some_function(&self) {
///         Event::emit (
///             MyEvents::StringEvent(String::from("some_string"))
///         )
///     }
///
///     pub fn another_function(&self) {
///         MyEvents::StringEvent(String::from("another_string")).emit()
///     }
/// }
/// ```
pub(crate) fn near_events(item: TokenStream) -> TokenStream {
    if let Ok(mut input) = syn::parse::<ItemEnum>(item) {
        // NearEvent Macro handles implementation
        input.attrs.push(parse_quote! (#[derive(near_sdk::serde::Serialize, std::clone::Clone, near_sdk::EventMetadata)]));
        input.attrs.push(parse_quote! (#[serde(crate="near_sdk::serde")]));
        input.attrs.push(parse_quote! (#[serde(tag = "event", content = "data")]));
        input.attrs.push(parse_quote! (#[serde(rename_all = "snake_case")]));

        TokenStream::from(quote! {
            #input
        })
    } else {
        TokenStream::from(
            syn::Error::new(
                Span::call_site(),
                "`near_events` can only be used as an attribute on enums.",
            )
            .to_compile_error(),
        )
    }
}
