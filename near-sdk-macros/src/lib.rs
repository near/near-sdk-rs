#![recursion_limit = "128"]
extern crate proc_macro;

mod core_impl;

use core_impl::{ext::generate_ext_structs, metadata::generate_contract_metadata_method};

use proc_macro::TokenStream;

use self::core_impl::*;
use proc_macro2::{Ident, Span};
use quote::{quote, ToTokens};
use syn::{parse_quote, ImplItem, ItemEnum, ItemImpl, ItemStruct, ItemTrait, WhereClause};

/// This attribute macro is used on a struct and its implementations
/// to generate the necessary code to expose `pub` methods from the contract as well
/// as generating the glue code to be a valid NEAR contract.
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
/// use near_sdk::near_bindgen;
///
/// #[near_bindgen]
/// pub struct Contract {
///    data: i8,
/// }
///
/// #[near_bindgen]
/// impl Contract {
///     pub fn some_function(&self) {}
/// }
/// ```
///
/// Events Standard:
///
/// By passing `event_json` as an argument `near_bindgen` will generate the relevant code to format events
/// according to NEP-297
///
/// For parameter serialization, this macro will generate a wrapper struct to include the NEP-297 standard fields `standard` and `version
/// as well as include serialization reformatting to include the `event` and `data` fields automatically.
/// The `standard` and `version` values must be included in the enum and variant declaration (see example below).
/// By default this will be JSON deserialized with `serde`
///
///
/// # Examples
///
/// ```ignore
/// use near_sdk::near_bindgen;
///
/// #[near_bindgen(event_json(standard = "nepXXX"))]
/// pub enum MyEvents {
///    #[event_version("1.0.0")]
///    Swap { token_in: AccountId, token_out: AccountId, amount_in: u128, amount_out: u128 },
///
///    #[event_version("2.0.0")]
///    StringEvent(String),
///
///    #[event_version("3.0.0")]
///    EmptyEvent
/// }
///
/// #[near_bindgen]
/// impl Contract {
///     pub fn some_function(&self) {
///         MyEvents::StringEvent(
///             String::from("some_string")
///         ).emit();
///     }
///
/// }
/// ```
///
/// Contract Source Metadata Standard:
///
/// By using `contract_metadata` as an argument `near_bindgen` will populate the contract metadata
/// according to [`NEP-330`](<https://github.com/near/NEPs/blob/master/neps/nep-0330.md>) standard. This still applies even when `#[near_bindgen]` is used without
/// any arguments.
///
/// All fields(version, link, standard) are optional and will be populated with defaults from the Cargo.toml file if not specified.
///
/// The `contract_source_metadata()` view function will be added and can be used to retrieve the source metadata.
/// Also, the source metadata will be stored as a constant, `CONTRACT_SOURCE_METADATA`, in the contract code.
///
/// # Examples
/// ```ignore
/// use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
/// use near_sdk::near_bindgen;
///
/// #[derive(Default, BorshSerialize, BorshDeserialize)]
/// #[near_bindgen(contract_metadata(
///     version = "39f2d2646f2f60e18ab53337501370dc02a5661c",
///     link = "https://github.com/near-examples/nft-tutorial",
///     standard(standard = "nep330", version = "1.1.0"),
///     standard(standard = "nep171", version = "1.0.0"),
///     standard(standard = "nep177", version = "2.0.0"),
/// ))]
/// struct Contract {}
/// ```
#[proc_macro_attribute]
pub fn near_bindgen(attr: TokenStream, item: TokenStream) -> TokenStream {
    if attr.to_string().contains("event_json") {
        return core_impl::near_events(attr, item);
    }

    let generate_metadata = |ident: &Ident,
                             generics: &syn::Generics|
     -> Result<proc_macro2::TokenStream, proc_macro2::TokenStream> {
        let metadata_impl_gen = generate_contract_metadata_method(ident, generics).into();
        let metadata_impl_gen = syn::parse::<ItemImpl>(metadata_impl_gen)
            .expect("failed to generate contract metadata");
        process_impl_block(metadata_impl_gen)
    };

    if let Ok(input) = syn::parse::<ItemStruct>(item.clone()) {
        let metadata = core_impl::contract_source_metadata_const(attr);

        let metadata_impl_gen = generate_metadata(&input.ident, &input.generics);

        let metadata_impl_gen = match metadata_impl_gen {
            Ok(metadata) => metadata,
            Err(err) => return err.into(),
        };

        let ext_gen = generate_ext_structs(&input.ident, Some(&input.generics));
        #[cfg(feature = "__abi-embed-checked")]
        let abi_embedded = abi::embed();
        #[cfg(not(feature = "__abi-embed-checked"))]
        let abi_embedded = quote! {};
        TokenStream::from(quote! {
            #input
            #ext_gen
            #abi_embedded
            #metadata
            #metadata_impl_gen
        })
    } else if let Ok(input) = syn::parse::<ItemEnum>(item.clone()) {
        let metadata = core_impl::contract_source_metadata_const(attr);
        let metadata_impl_gen = generate_metadata(&input.ident, &input.generics);

        let metadata_impl_gen = match metadata_impl_gen {
            Ok(metadata) => metadata,
            Err(err) => return err.into(),
        };

        let ext_gen = generate_ext_structs(&input.ident, Some(&input.generics));
        #[cfg(feature = "__abi-embed-checked")]
        let abi_embedded = abi::embed();
        #[cfg(not(feature = "__abi-embed-checked"))]
        let abi_embedded = quote! {};
        TokenStream::from(quote! {
            #input
            #ext_gen
            #abi_embedded
            #metadata
            #metadata_impl_gen
        })
    } else if let Ok(input) = syn::parse::<ItemImpl>(item) {
        for method in &input.items {
            if let ImplItem::Fn(m) = method {
                let ident = &m.sig.ident;
                if ident.eq("__contract_abi") || ident.eq("contract_source_metadata") {
                    return TokenStream::from(
                        syn::Error::new_spanned(
                            ident.to_token_stream(),
                            "use of reserved contract method",
                        )
                        .to_compile_error(),
                    );
                }
            }
        }
        match process_impl_block(input) {
            Ok(output) => output,
            Err(output) => output,
        }
        .into()
    } else {
        TokenStream::from(
            syn::Error::new(
                Span::call_site(),
                "near_bindgen can only be used on struct or enum definition and impl sections.",
            )
            .to_compile_error(),
        )
    }
}

// This function deals with impl block processing, generating wrappers and ABI.
//
// # Arguments
// * input - impl block to process.
//
// The Result has a TokenStream error type, because those need to be propagated to the compiler.
fn process_impl_block(
    mut input: ItemImpl,
) -> Result<proc_macro2::TokenStream, proc_macro2::TokenStream> {
    let item_impl_info = match ItemImplInfo::new(&mut input) {
        Ok(x) => x,
        Err(err) => return Err(err.to_compile_error()),
    };

    #[cfg(not(feature = "__abi-generate"))]
    let abi_generated = quote! {};
    #[cfg(feature = "__abi-generate")]
    let abi_generated = abi::generate(&item_impl_info);

    let generated_code = item_impl_info.wrapper_code();

    // Add wrapper methods for ext call API
    let ext_generated_code = item_impl_info.generate_ext_wrapper_code();

    Ok(TokenStream::from(quote! {
        #ext_generated_code
        #input
        #generated_code
        #abi_generated
    })
    .into())
}

/// `ext_contract` takes a Rust Trait and converts it to a module with static methods.
/// Each of these static methods takes positional arguments defined by the Trait,
/// then the receiver_id, the attached deposit and the amount of gas and returns a new Promise.
///
/// # Examples
///
/// ```ignore
/// use near_sdk::ext_contract;
///
/// #[ext_contract(ext_calculator)]
/// trait Calculator {
///     fn mult(&self, a: u64, b: u64) -> u128;
///     fn sum(&self, a: u128, b: u128) -> u128;
/// }
/// ```
#[proc_macro_attribute]
pub fn ext_contract(attr: TokenStream, item: TokenStream) -> TokenStream {
    if let Ok(mut input) = syn::parse::<ItemTrait>(item) {
        let mod_name: Option<proc_macro2::Ident> = if attr.is_empty() {
            None
        } else {
            match syn::parse(attr) {
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
            }
        };
        let item_trait_info = match ItemTraitInfo::new(&mut input, mod_name) {
            Ok(x) => x,
            Err(err) => return TokenStream::from(err.to_compile_error()),
        };
        let ext_api = item_trait_info.wrap_trait_ext();

        TokenStream::from(quote! {
            #input
            #ext_api
        })
    } else {
        TokenStream::from(
            syn::Error::new(Span::call_site(), "ext_contract can only be used on traits")
                .to_compile_error(),
        )
    }
}

// The below attributes a marker-attributes and therefore they are no-op.

/// `callback` is a marker attribute it does not generate code by itself.
#[proc_macro_attribute]
#[deprecated(since = "4.0.0", note = "Case is handled internally by macro, no need to import")]
pub fn callback(_attr: TokenStream, item: TokenStream) -> TokenStream {
    item
}

/// `callback_args_vec` is a marker attribute it does not generate code by itself.
#[deprecated(since = "4.0.0", note = "Case is handled internally by macro, no need to import")]
#[proc_macro_attribute]
pub fn callback_vec(_attr: TokenStream, item: TokenStream) -> TokenStream {
    item
}

/// `serializer` is a marker attribute it does not generate code by itself.
#[deprecated(since = "4.0.0", note = "Case is handled internally by macro, no need to import")]
#[proc_macro_attribute]
pub fn serializer(_attr: TokenStream, item: TokenStream) -> TokenStream {
    item
}

/// `result_serializer` is a marker attribute it does not generate code by itself.
#[deprecated(since = "4.0.0", note = "Case is handled internally by macro, no need to import")]
#[proc_macro_attribute]
pub fn result_serializer(_attr: TokenStream, item: TokenStream) -> TokenStream {
    item
}

/// `init` is a marker attribute it does not generate code by itself.
#[deprecated(since = "4.0.0", note = "Case is handled internally by macro, no need to import")]
#[proc_macro_attribute]
pub fn init(_attr: TokenStream, item: TokenStream) -> TokenStream {
    item
}

#[cfg(feature = "abi")]
use darling::FromDeriveInput;
#[derive(darling::FromDeriveInput, Debug)]
#[darling(attributes(abi), forward_attrs(serde, borsh_skip, schemars, validate))]
#[cfg(feature = "abi")]
struct DeriveNearSchema {
    attrs: Vec<syn::Attribute>,
    json: Option<bool>,
    borsh: Option<bool>,
}

#[cfg(feature = "abi")]
#[proc_macro_derive(NearSchema, attributes(abi, serde, borsh, schemars, validate))]
pub fn derive_near_schema(input: TokenStream) -> TokenStream {
    let derive_input = syn::parse_macro_input!(input as syn::DeriveInput);
    let args = match DeriveNearSchema::from_derive_input(&derive_input) {
        Ok(v) => v,
        Err(e) => {
            return TokenStream::from(e.write_errors());
        }
    };

    if args.borsh.is_none()
        && args.json.is_none()
        && derive_input.attrs.iter().any(|attr| attr.path().is_ident("abi"))
    {
        return TokenStream::from(
            syn::Error::new_spanned(
                derive_input.to_token_stream(),
                "At least one of `json` or `borsh` inside of `#[abi(...)]` must be specified",
            )
            .to_compile_error(),
        );
    }

    // #[abi(json, borsh)]
    let (json_schema, borsh_schema) = (args.json.unwrap_or(false), args.borsh.unwrap_or(false));
    let mut input = derive_input;
    input.attrs = args.attrs;

    let strip_unknown_attr = |attrs: &mut Vec<syn::Attribute>| {
        attrs.retain(|attr| {
            ["serde", "schemars", "validate", "borsh"]
                .iter()
                .any(|&path| attr.path().is_ident(path))
        });
    };

    match &mut input.data {
        syn::Data::Struct(data) => {
            for field in &mut data.fields {
                strip_unknown_attr(&mut field.attrs);
            }
        }
        syn::Data::Enum(data) => {
            for variant in &mut data.variants {
                strip_unknown_attr(&mut variant.attrs);
                for field in &mut variant.fields {
                    strip_unknown_attr(&mut field.attrs);
                }
            }
        }
        syn::Data::Union(_) => {
            return TokenStream::from(
                syn::Error::new_spanned(
                    input.to_token_stream(),
                    "`NearSchema` does not support derive for unions",
                )
                .to_compile_error(),
            )
        }
    }

    // <unspecified> or #[abi(json)]
    let json_schema = json_schema || !borsh_schema;

    let derive = {
        let mut derive = quote! {};
        if borsh_schema {
            derive = quote! {
                #[derive(::near_sdk::borsh::BorshSchema)]
                #[borsh(crate = "::near_sdk::borsh")]
            };
        }
        if json_schema {
            derive = quote! {
                #derive
                #[derive(::near_sdk::schemars::JsonSchema)]
                #[schemars(crate = "::near_sdk::schemars")]
            };
        }
        derive
    };

    let input_ident = &input.ident;

    let input_ident_proxy = quote::format_ident!("{}__NEAR_SCHEMA_PROXY", input_ident);

    let json_impl = if json_schema {
        quote! {
            #[automatically_derived]
            impl ::near_sdk::schemars::JsonSchema for #input_ident_proxy {
                fn schema_name() -> ::std::string::String {
                    stringify!(#input_ident).to_string()
                }

                fn json_schema(gen: &mut ::near_sdk::schemars::gen::SchemaGenerator) -> ::near_sdk::schemars::schema::Schema {
                    <#input_ident as ::near_sdk::schemars::JsonSchema>::json_schema(gen)
                }
            }
        }
    } else {
        quote! {}
    };

    let borsh_impl = if borsh_schema {
        quote! {
            #[automatically_derived]
            impl ::near_sdk::borsh::BorshSchema for #input_ident_proxy {
                fn declaration() -> ::near_sdk::borsh::schema::Declaration {
                    stringify!(#input_ident).to_string()
                }

                fn add_definitions_recursively(
                    definitions: &mut ::near_sdk::borsh::__private::maybestd::collections::BTreeMap<
                        ::near_sdk::borsh::schema::Declaration,
                        ::near_sdk::borsh::schema::Definition
                    >,
                ) {
                    <#input_ident as ::near_sdk::borsh::BorshSchema>::add_definitions_recursively(definitions);
                }
            }
        }
    } else {
        quote! {}
    };

    TokenStream::from(quote! {
        #[cfg(not(target_arch = "wasm32"))]
        const _: () = {
            #[allow(non_camel_case_types)]
            type #input_ident_proxy = #input_ident;
            {
                #derive
                #input

                #json_impl
                #borsh_impl
            };
        };
    })
}

/// `PanicOnDefault` generates implementation for `Default` trait that panics with the following
/// message `The contract is not initialized` when `default()` is called.
/// This is a helpful macro in case the contract is required to be initialized with either `init` or
/// `init(ignore_state)`.
#[proc_macro_derive(PanicOnDefault)]
pub fn derive_no_default(item: TokenStream) -> TokenStream {
    if let Ok(input) = syn::parse::<ItemStruct>(item) {
        let name = &input.ident;
        TokenStream::from(quote! {
            impl ::std::default::Default for #name {
                fn default() -> Self {
                    ::near_sdk::env::panic_str("The contract is not initialized");
                }
            }
        })
    } else {
        TokenStream::from(
            syn::Error::new(
                Span::call_site(),
                "PanicOnDefault can only be used on type declarations sections.",
            )
            .to_compile_error(),
        )
    }
}

/// `BorshStorageKey` generates implementation for `BorshIntoStorageKey` trait.
/// It allows the type to be passed as a unique prefix for persistent collections.
/// The type should also implement or derive `BorshSerialize` trait.
#[proc_macro_derive(BorshStorageKey)]
pub fn borsh_storage_key(item: TokenStream) -> TokenStream {
    let (name, generics) = if let Ok(input) = syn::parse::<ItemEnum>(item.clone()) {
        (input.ident, input.generics)
    } else if let Ok(input) = syn::parse::<ItemStruct>(item) {
        (input.ident, input.generics)
    } else {
        return TokenStream::from(
            syn::Error::new(
                Span::call_site(),
                "BorshStorageKey can only be used as a derive on enums or structs.",
            )
            .to_compile_error(),
        );
    };
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    let predicate = parse_quote!(#name #ty_generics: ::near_sdk::borsh::BorshSerialize);
    let where_clause: WhereClause = if let Some(mut w) = where_clause.cloned() {
        w.predicates.push(predicate);
        w
    } else {
        parse_quote!(where #predicate)
    };
    TokenStream::from(quote! {
        impl #impl_generics ::near_sdk::__private::BorshIntoStorageKey for #name #ty_generics #where_clause {}
    })
}

/// `FunctionError` generates implementation for `near_sdk::FunctionError` trait.
/// It allows contract runtime to panic with the type using its `ToString` implementation
/// as the message.
#[proc_macro_derive(FunctionError)]
pub fn function_error(item: TokenStream) -> TokenStream {
    let name = if let Ok(input) = syn::parse::<ItemEnum>(item.clone()) {
        input.ident
    } else if let Ok(input) = syn::parse::<ItemStruct>(item) {
        input.ident
    } else {
        return TokenStream::from(
            syn::Error::new(
                Span::call_site(),
                "FunctionError can only be used as a derive on enums or structs.",
            )
            .to_compile_error(),
        );
    };
    TokenStream::from(quote! {
        impl ::near_sdk::FunctionError for #name {
            fn panic(&self) -> ! {
                ::near_sdk::env::panic_str(&::std::string::ToString::to_string(&self))
            }
        }
    })
}

/// NOTE: This is an internal implementation for `#[near_bindgen(events(standard = ...))]` attribute.
///
/// This derive macro is used to inject the necessary wrapper and logic to auto format
/// standard event logs. The other appropriate attribute macros are not injected with this macro.
/// Required attributes below:
/// ```ignore
/// #[derive(near_sdk::serde::Serialize, std::clone::Clone)]
/// #[serde(crate="near_sdk::serde")]
/// #[serde(tag = "event", content = "data")]
/// #[serde(rename_all="snake_case")]
/// pub enum MyEvent {
///     Event
/// }
/// ```
#[proc_macro_derive(EventMetadata, attributes(event_version))]
pub fn derive_event_attributes(item: TokenStream) -> TokenStream {
    if let Ok(input) = syn::parse::<ItemEnum>(item) {
        let name = &input.ident;
        // get `standard` const injected from `near_events`
        let standard_name = format!("{}_event_standard", name);
        let standard_ident = syn::Ident::new(&standard_name, Span::call_site());
        // version from each attribute macro
        let mut event_meta: Vec<proc_macro2::TokenStream> = vec![];
        for var in &input.variants {
            if let Some(version) = core_impl::get_event_version(var) {
                let var_ident = &var.ident;
                event_meta.push(quote! {
                    #name::#var_ident { .. } => {(::std::string::ToString::to_string(&#standard_ident), ::std::string::ToString::to_string(#version))}
                })
            } else {
                return TokenStream::from(
                    syn::Error::new(
                        Span::call_site(),
                        "Near events must have `event_version`. Must have a single string literal value.",
                    )
                    .to_compile_error(),
                );
            }
        }

        // handle lifetimes, generics, and where clauses
        let (impl_generics, type_generics, where_clause) = &input.generics.split_for_impl();
        // add `'near_event` lifetime for user defined events
        let mut generics = input.generics.clone();
        let event_lifetime = syn::Lifetime::new("'near_event", Span::call_site());
        generics.params.insert(
            0,
            syn::GenericParam::Lifetime(syn::LifetimeParam::new(event_lifetime.clone())),
        );
        let (custom_impl_generics, ..) = generics.split_for_impl();

        TokenStream::from(quote! {
            impl #impl_generics #name #type_generics #where_clause {
                pub fn emit(&self) {
                    use ::std::string::String;

                    let (standard, version): (String, String) = match self {
                        #(#event_meta),*
                    };

                    #[derive(::near_sdk::serde::Serialize)]
                    #[serde(crate="::near_sdk::serde")]
                    #[serde(rename_all="snake_case")]
                    struct EventBuilder #custom_impl_generics #where_clause {
                        standard: String,
                        version: String,
                        #[serde(flatten)]
                        event_data: &#event_lifetime #name #type_generics
                    }
                    let event = EventBuilder { standard, version, event_data: self };
                    let json = ::near_sdk::serde_json::to_string(&event)
                            .unwrap_or_else(|_| ::near_sdk::env::abort());
                    ::near_sdk::env::log_str(&::std::format!("EVENT_JSON:{}", json));
                }
            }
        })
    } else {
        TokenStream::from(
            syn::Error::new(
                Span::call_site(),
                "EventMetadata can only be used as a derive on enums.",
            )
            .to_compile_error(),
        )
    }
}
