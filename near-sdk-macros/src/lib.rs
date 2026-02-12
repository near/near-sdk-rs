#![recursion_limit = "128"]
extern crate proc_macro;

mod core_impl;

use core_impl::{ext::generate_ext_structs, metadata::generate_contract_metadata_method};

use proc_macro::TokenStream;

use self::core_impl::*;
use darling::ast::NestedMeta;
use darling::{Error, FromMeta};
use inflector::Inflector;
use proc_macro2::{Ident, Span};
use quote::{ToTokens, quote};
use syn::{
    Expr, ImplItem, Item, ItemEnum, ItemImpl, ItemStruct, ItemTrait, WhereClause, parse_quote,
};

#[derive(Debug, Clone)]
struct Serializers {
    vec: Vec<Expr>,
}

impl FromMeta for Serializers {
    fn from_expr(expr: &syn::Expr) -> Result<Self, darling::Error> {
        match expr {
            syn::Expr::Array(expr_array) => Ok(Serializers {
                vec: expr_array
                    .elems
                    .iter()
                    .map(<Expr as FromMeta>::from_expr)
                    .map(|x| x.unwrap())
                    .collect::<Vec<_>>(),
            }),
            _ => Err(Error::unexpected_expr_type(expr)),
        }
    }
}

#[derive(FromMeta)]
struct NearMacroArgs {
    serializers: Option<Serializers>,
    #[darling(default, flatten)]
    near_bindgen_args: NearBindgenMacroArgs,
    inside_nearsdk: Option<bool>,
}

fn has_nested_near_macros(item: TokenStream) -> bool {
    syn::parse::<syn::Item>(item)
        .ok()
        .and_then(|item_ast| {
            let attrs = match item_ast {
                syn::Item::Struct(s) => s.attrs,
                syn::Item::Enum(e) => e.attrs,
                syn::Item::Impl(i) => i.attrs,
                _ => vec![], // Other cases don't support near macros anyway
            };

            attrs.into_iter().find(|attr| {
                let path_str = attr.path().to_token_stream().to_string();
                path_str == "near" || path_str == "near_bindgen"
            })
        })
        .is_some()
}

#[proc_macro_attribute]
pub fn near(attr: TokenStream, item: TokenStream) -> TokenStream {
    if attr.to_string().contains("event_json") {
        return core_impl::near_events(attr, item);
    }

    let meta_list = match NestedMeta::parse_meta_list(attr.into()) {
        Ok(v) => v,
        Err(e) => {
            return TokenStream::from(Error::from(e).write_errors());
        }
    };

    let near_macro_args = match NearMacroArgs::from_list(&meta_list) {
        Ok(v) => v,
        Err(e) => {
            return TokenStream::from(e.write_errors());
        }
    };

    let near_sdk_crate = if near_macro_args.inside_nearsdk.unwrap_or(false) {
        quote! {crate}
    } else {
        quote! {::near_sdk}
    };

    // Check for nested near macros by parsing the input and examining actual attributes
    if has_nested_near_macros(item.clone()) {
        return TokenStream::from(
            syn::Error::new(
                Span::call_site(),
                "#[near] or #[near_bindgen] attributes are not allowed to be nested inside of the outmost #[near] attribute. Only a single #[near] attribute is allowed",
            )
            .to_compile_error(),
        );
    }
    let string_borsh_crate = quote! {#near_sdk_crate::borsh}.to_string();
    let string_serde_crate = quote! {#near_sdk_crate::serde}.to_string();
    let string_serde_with_crate = quote! {#near_sdk_crate::serde_with}.to_string();

    let mut expanded: proc_macro2::TokenStream = quote! {};

    if near_macro_args.near_bindgen_args.contract_state.is_some() {
        let near_bindgen_args = near_macro_args.near_bindgen_args;
        expanded = quote! {#[#near_sdk_crate::near_bindgen(#near_bindgen_args)]};
    };

    let mut has_borsh = false;
    let mut has_json = false;

    let mut borsh_attr = quote! {};

    match near_macro_args.serializers {
        Some(serializers) => {
            let attr2 = serializers.clone();

            attr2.vec.iter().for_each(|old_expr| {
                let new_expr = &mut old_expr.clone();
                match &mut *new_expr {
                    Expr::Call(call_expr) => {
                        if let Expr::Path(path) = &mut *call_expr.func {
                            if let Some(ident) = path.path.get_ident() {
                                if *ident == "json" {
                                    has_json = true;
                                    path.path =
                                        syn::Path::from(Ident::new("serde", Span::call_site()));
                                    call_expr.args.push(parse_quote! {crate=#string_serde_crate});
                                } else if *ident == "borsh" {
                                    has_borsh = true;
                                    call_expr.args.push(parse_quote! {crate=#string_borsh_crate});
                                }
                            }
                        }
                        borsh_attr = quote! {#[#new_expr]};
                    }
                    Expr::Path(path_expr) => {
                        if let Some(ident) = path_expr.path.get_ident() {
                            if *ident == "json" {
                                has_json = true;
                            }
                            if *ident == "borsh" {
                                has_borsh = true;
                                borsh_attr = quote! {#[borsh(crate=#string_borsh_crate)]};
                            }
                        }
                    }
                    _ => {}
                }
            });
        }
        None => {
            has_borsh = true;
            borsh_attr = quote! {#[borsh(crate = #string_borsh_crate)]};
        }
    }

    // Add serde_as BEFORE schema_derive so that serde_with can process
    // the schemars = true parameter and inject schema attributes before
    // JsonSchema derive macro runs
    if has_json {
        let enable_schemars = cfg!(feature = "abi").then_some(quote! {schemars = true,});
        expanded = quote! {
            #expanded
            #[#near_sdk_crate::serde_with::serde_as(
                crate = #string_serde_with_crate,
                #enable_schemars
            )]
            #[derive(#near_sdk_crate::serde::Serialize, #near_sdk_crate::serde::Deserialize)]
            #[serde(crate = #string_serde_crate)]
        };
    }

    #[cfg(feature = "abi")]
    {
        let schema_derive: proc_macro2::TokenStream =
            get_schema_derive(has_json, has_borsh, near_sdk_crate.clone(), false);
        expanded = quote! {
            #expanded
            #schema_derive
        };
    }

    if has_borsh {
        expanded = quote! {
            #expanded
            #[derive(#near_sdk_crate::borsh::BorshSerialize, #near_sdk_crate::borsh::BorshDeserialize)]
            #borsh_attr
        };
    }

    if let Ok(input) = syn::parse::<ItemStruct>(item.clone()) {
        expanded = quote! {
            #expanded
            #input
        };
    } else if let Ok(input) = syn::parse::<ItemEnum>(item.clone()) {
        expanded = quote! {
            #expanded
            #input
        };
    } else if let Ok(input) = syn::parse::<ItemImpl>(item) {
        expanded = quote! {
            #[#near_sdk_crate::near_bindgen]
            #input
        };
    } else {
        return TokenStream::from(
            syn::Error::new(
                Span::call_site(),
                "near macro can only be used on struct or enum definition and impl sections.",
            )
            .to_compile_error(),
        );
    }

    TokenStream::from(expanded)
}

#[derive(Default, FromMeta)]
struct NearBindgenMacroArgs {
    contract_state: Option<core_impl::state::ContractStateArgs>,
    #[darling(default)]
    contract_metadata: core_impl::ContractMetadata,
}

impl quote::ToTokens for NearBindgenMacroArgs {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let contract_state = &self.contract_state;
        let contract_metadata = &self.contract_metadata;
        tokens.extend(quote! {
            contract_state(#contract_state),
            contract_metadata(#contract_metadata),
        })
    }
}

#[proc_macro_attribute]
pub fn near_bindgen(attr: TokenStream, item: TokenStream) -> TokenStream {
    if attr.to_string().contains("event_json") {
        return core_impl::near_events(attr, item);
    }

    let (ident, generics) = if let Ok(input) = syn::parse::<ItemStruct>(item.clone()) {
        (input.ident, input.generics)
    } else if let Ok(input) = syn::parse::<ItemEnum>(item.clone()) {
        (input.ident, input.generics)
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
        return match process_impl_block(input) {
            Ok(output) => output,
            Err(output) => output,
        }
        .into();
    } else {
        return TokenStream::from(
            syn::Error::new(
                Span::call_site(),
                "near_bindgen can only be used on struct or enum definition and impl sections.",
            )
            .to_compile_error(),
        );
    };

    let attr_args = match NestedMeta::parse_meta_list(attr.into()) {
        Ok(v) => v,
        Err(e) => {
            return Error::from(e).write_errors().into();
        }
    };

    let args = match NearBindgenMacroArgs::from_list(&attr_args) {
        Ok(v) => v,
        Err(e) => {
            return e.write_errors().into();
        }
    };

    let impl_contract_state = args.contract_state.map(|s| s.impl_contract_state(&ident, &generics));
    let metadata = args.contract_metadata.contract_source_metadata_const();

    let metadata_impl_gen = {
        let metadata_impl_gen = generate_contract_metadata_method(&ident, &generics).into();

        let metadata_impl_gen = syn::parse::<ItemImpl>(metadata_impl_gen)
            .expect("failed to generate contract metadata");
        process_impl_block(metadata_impl_gen)
    };

    let metadata_impl_gen = match metadata_impl_gen {
        Ok(metadata) => metadata,
        Err(err) => return err.into(),
    };

    let ext_gen = generate_ext_structs(&ident, Some(&generics));
    #[cfg(feature = "__abi-embed-checked")]
    let abi_embedded = abi::embed();
    #[cfg(not(feature = "__abi-embed-checked"))]
    let abi_embedded = quote! {};
    let item = proc_macro2::TokenStream::from(item);
    TokenStream::from(quote! {
        #item
        #ext_gen
        #abi_embedded
        #metadata
        #metadata_impl_gen
        #impl_contract_state
    })
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

    let error_methods = item_impl_info.generate_error_method();

    Ok(TokenStream::from(quote! {
        #ext_generated_code
        #input
        #generated_code
        #abi_generated
        #error_methods
    })
    .into())
}

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
                            format!("Failed to parse mod name for ext_contract: {err}"),
                        )
                        .to_compile_error(),
                    );
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

#[cfg(feature = "abi")]
#[derive(darling::FromDeriveInput, Debug)]
#[darling(attributes(abi), forward_attrs(serde, borsh_skip, schemars, validate))]
struct DeriveNearSchema {
    attrs: Vec<syn::Attribute>,
    json: Option<bool>,
    borsh: Option<bool>,
}

#[proc_macro_derive(NearSchema, attributes(abi, serde, borsh, schemars, validate, inside_nearsdk))]
pub fn derive_near_schema(#[allow(unused)] input: TokenStream) -> TokenStream {
    #[cfg(not(feature = "abi"))]
    {
        TokenStream::from(quote! {})
    }

    #[cfg(feature = "abi")]
    {
        use darling::FromDeriveInput;

        let derive_input = syn::parse_macro_input!(input as syn::DeriveInput);
        let generics = derive_input.generics.clone();
        let args = match DeriveNearSchema::from_derive_input(&derive_input) {
            Ok(v) => v,
            Err(e) => {
                return TokenStream::from(e.write_errors());
            }
        };

        if args.borsh.is_none()
            && args.json.is_none()
            && derive_input.clone().attrs.iter().any(|attr| attr.path().is_ident("abi"))
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
        let mut input = derive_input.clone();
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
                );
            }
        }

        let near_sdk_crate =
            if derive_input.attrs.iter().any(|attr| attr.path().is_ident("inside_nearsdk")) {
                quote! {crate}
            } else {
                quote! {::near_sdk}
            };

        // <unspecified> or #[abi(json)]
        let json_schema = json_schema || !borsh_schema;

        let derive = get_schema_derive(json_schema, borsh_schema, near_sdk_crate.clone(), true);

        let input_ident = &input.ident;

        let input_ident_proxy = quote::format_ident!("{}__NEAR_SCHEMA_PROXY", input_ident);

        let json_impl = if json_schema {
            let where_clause = get_where_clause(
                &generics,
                input_ident,
                quote! {#near_sdk_crate::schemars::JsonSchema},
            );
            quote! {
                #[automatically_derived]
                impl #generics #near_sdk_crate::schemars::JsonSchema for #input_ident_proxy #generics #where_clause {
                    fn schema_name() -> ::std::string::String {
                        <#input_ident #generics as #near_sdk_crate::schemars::JsonSchema>::schema_name()
                    }

                    fn json_schema(r#gen: &mut #near_sdk_crate::schemars::r#gen::SchemaGenerator) -> #near_sdk_crate::schemars::schema::Schema {
                        <#input_ident #generics as #near_sdk_crate::schemars::JsonSchema>::json_schema(r#gen)
                    }
                }
            }
        } else {
            quote! {}
        };

        let borsh_impl = if borsh_schema {
            let where_clause = get_where_clause(
                &generics,
                input_ident,
                quote! {#near_sdk_crate::borsh::BorshSchema},
            );
            quote! {
                #[automatically_derived]
                impl #generics #near_sdk_crate::borsh::BorshSchema for #input_ident_proxy #generics #where_clause {
                    fn declaration() -> #near_sdk_crate::borsh::schema::Declaration {
                        <#input_ident #generics as #near_sdk_crate::borsh::BorshSchema>::declaration()
                    }

                    fn add_definitions_recursively(
                        definitions: &mut #near_sdk_crate::borsh::__private::maybestd::collections::BTreeMap<
                            #near_sdk_crate::borsh::schema::Declaration,
                            #near_sdk_crate::borsh::schema::Definition
                        >,
                    ) {
                        <#input_ident #generics as #near_sdk_crate::borsh::BorshSchema>::add_definitions_recursively(definitions);
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
                type #input_ident_proxy #generics = #input_ident #generics;
                {
                    #derive
                    #[allow(dead_code)]
                    #input

                    #json_impl
                    #borsh_impl
                };
            };
        })
    }
}

#[allow(dead_code)]
fn get_schema_derive(
    json_schema: bool,
    borsh_schema: bool,
    near_sdk_crate: proc_macro2::TokenStream,
    need_borsh_crate: bool,
) -> proc_macro2::TokenStream {
    let string_borsh_crate = quote! {#near_sdk_crate::borsh}.to_string();
    let string_schemars_crate = quote! {#near_sdk_crate::schemars}.to_string();

    let mut derive = quote! {};
    if borsh_schema {
        derive = quote! {
            #[cfg_attr(not(target_arch = "wasm32"), derive(#near_sdk_crate::borsh::BorshSchema))]
        };
        if need_borsh_crate {
            derive = quote! {
                #derive
                #[cfg_attr(not(target_arch = "wasm32"), borsh(crate = #string_borsh_crate))]
            };
        }
    }
    if json_schema {
        derive = quote! {
            #derive
            #[cfg_attr(not(target_arch = "wasm32"), derive(#near_sdk_crate::schemars::JsonSchema))]
            #[cfg_attr(not(target_arch = "wasm32"), schemars(crate = #string_schemars_crate))]
        };
    }
    derive
}

#[cfg(feature = "abi")]
fn get_where_clause(
    generics: &syn::Generics,
    input_ident: &syn::Ident,
    trait_name: proc_macro2::TokenStream,
) -> WhereClause {
    let (_, ty_generics, where_clause) = generics.split_for_impl();

    let predicate = parse_quote!(#input_ident #ty_generics: #trait_name);

    let where_clause: WhereClause = if let Some(mut w) = where_clause.cloned() {
        w.predicates.push(predicate);
        w
    } else {
        parse_quote!(where #predicate)
    };
    where_clause
}

#[proc_macro_derive(PanicOnDefault)]
pub fn derive_no_default(item: TokenStream) -> TokenStream {
    match syn::parse::<Item>(item) {
        Ok(Item::Enum(ItemEnum { ident, .. })) | Ok(Item::Struct(ItemStruct { ident, .. })) => {
            TokenStream::from(quote! {
                const _: () = {
                    #[automatically_derived]
                    impl ::std::default::Default for #ident {
                        fn default() -> Self {
                            ::near_sdk::env::panic_err(::near_sdk::errors::ContractNotInitialized{});
                        }
                    }
                };
            })
        }
        _ => TokenStream::from(
            syn::Error::new(
                Span::call_site(),
                "PanicOnDefault can only be used on type declarations sections.",
            )
            .to_compile_error(),
        ),
    }
}

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
                ::near_sdk::env::panic_err(::near_sdk::errors::ContractError::new(&::std::string::ToString::to_string(&self)))
            }
        }
    })
}

#[proc_macro_derive(EventMetadata, attributes(event_version))]
pub fn derive_event_attributes(item: TokenStream) -> TokenStream {
    if let Ok(input) = syn::parse::<ItemEnum>(item) {
        let name = &input.ident;
        // get `standard` const injected from `near_events`
        let standard_name = format!("{name}_event_standard");
        let standard_ident = syn::Ident::new(&standard_name, Span::call_site());
        // version from each attribute macro
        let mut event_meta: Vec<proc_macro2::TokenStream> = vec![];
        let mut version_arms: Vec<proc_macro2::TokenStream> = vec![];
        let mut event_name_arms: Vec<proc_macro2::TokenStream> = vec![];
        for var in &input.variants {
            if let Some(version) = core_impl::get_event_version(var) {
                let var_ident = &var.ident;
                event_meta.push(quote! {
                    #name::#var_ident { .. } => {(::std::string::ToString::to_string(&#standard_ident), ::std::string::ToString::to_string(#version))}
                });
                version_arms.push(quote! {
                    #name::#var_ident { .. } => ::std::borrow::Cow::Borrowed(#version)
                });
                let event_name =
                    proc_macro2::Literal::string(&var.ident.to_string().to_snake_case());
                event_name_arms.push(quote! {
                    #name::#var_ident { .. } => ::std::borrow::Cow::Borrowed(#event_name)
                });
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

        TokenStream::from(quote! {
            impl #impl_generics #name #type_generics #where_clause {
                pub fn emit(&self) {
                    use ::near_sdk::AsNep297Event;
                    ::near_sdk::env::log_str(&self.to_nep297_event().to_event_log());
                }

                pub fn to_json(&self) -> ::near_sdk::serde_json::Value {
                    use ::near_sdk::AsNep297Event;
                    self.to_nep297_event().to_json()
                }

                pub fn standard(&self) -> ::std::borrow::Cow<'_, str> {
                    ::std::borrow::Cow::Borrowed(#standard_ident)
                }

                pub fn version(&self) -> ::std::borrow::Cow<'_, str> {
                    match self {
                        #(#version_arms),*
                    }
                }

                pub fn event(&self) -> ::std::borrow::Cow<'_, str> {
                    match self {
                        #(#event_name_arms),*
                    }
                }
            }

            impl #impl_generics ::near_sdk::events::AsNep297Event for #name #type_generics #where_clause{
                fn to_nep297_event(&self) -> ::near_sdk::events::Nep297Event<'_, Self> {
                    ::near_sdk::events::Nep297Event::new(
                        ::std::borrow::Cow::Borrowed(#standard_ident),
                        match self {
                            #(#version_arms),*
                        },
                        match self {
                            #(#event_name_arms),*
                        },
                        self,
                    )
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

#[derive(FromMeta)]
struct ContractErrorArgs {
    sdk: Option<bool>,
    inside_nearsdk: Option<bool>,
}

/// This attribute macro is used on a struct or enum to generate the necessary code for an error
///  returned from a contract method call.
///
/// # Example:
///  
/// For the error:
/// ```ignore
/// #[contract_error]
/// pub struct MyError {
///    field1: String,
///    field2: String,
/// }
/// ```
///
/// And the function:
/// ```ignore
/// pub fn my_method(&self) -> Result<(), MyError>;
/// ```
///
/// The error is serialized as a JSON object with the following structure:
/// ```text
/// {
///    "error": {
///       // this name can be "SDK_CONTRACT_ERROR" and "CUSTOM_CONTRACT_ERROR"
///       // To generate "SDK_CONTRACT_ERROR", use sdk attribute `#[contract_error(sdk)]`.
///       // Otherwise, it will generate "CUSTOM_CONTRACT_ERROR"
///       "name": "CUSTOM_CONTRACT_ERROR",
///       "cause": {
///         // this name is the name of error struct
///         "name": "MyError",
///         "info": {
///             /// fields of the error struct
///             "field1": "value1",
///             "field2": "value2"
///         }
///    }
/// }
/// ```
///
/// Note: you can assign any error defined like that to BaseError:
/// ```ignore
/// let base_error: BaseError = MyError { field1: "value1".to_string(), field2: "value2".to_string() }.into();
/// ```
///
/// Use inside_nearsdk attribute (#[contract_error(inside_nearsdk)]) if the error struct is defined inside near-sdk.
/// Don't use if it is defined outside.
///
/// Internally, it makes error struct to:
///  - implement `near_sdk::ContractErrorTrait` so that it becomes correct error, which can be returned from contract method with defined structure.
///  - implement `From<ErrorStruct> for near_sdk::BaseError` as a polymorphic solution
///  - implement `From<ErrorStruct> for String` to convert the error to a string
#[proc_macro_attribute]
pub fn contract_error(attr: TokenStream, item: TokenStream) -> TokenStream {
    let meta_list = match NestedMeta::parse_meta_list(attr.into()) {
        Ok(v) => v,
        Err(e) => {
            return TokenStream::from(Error::from(e).write_errors());
        }
    };

    let contract_error_args = match ContractErrorArgs::from_list(&meta_list) {
        Ok(v) => v,
        Err(e) => {
            return TokenStream::from(e.write_errors());
        }
    };

    let input = syn::parse_macro_input!(item as syn::DeriveInput);
    let ident = &input.ident;
    let generics = &input.generics;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let error_type = if contract_error_args.sdk.unwrap_or(false) {
        quote! {"SDK_CONTRACT_ERROR"}
    } else {
        quote! {"CUSTOM_CONTRACT_ERROR"}
    };

    let near_sdk_crate: proc_macro2::TokenStream;
    let mut bool_inside_nearsdk_for_macro = quote! {false};

    if contract_error_args.inside_nearsdk.unwrap_or(false) {
        near_sdk_crate = quote! {crate};
        bool_inside_nearsdk_for_macro = quote! {true};
    } else {
        near_sdk_crate = quote! {::near_sdk};
    };

    let mut expanded = quote! {
        #[#near_sdk_crate ::near(serializers=[json], inside_nearsdk=#bool_inside_nearsdk_for_macro)]
        #[derive(Debug)]
        #input

        impl #impl_generics #near_sdk_crate ::ContractErrorTrait for #ident #ty_generics #where_clause {
            fn error_type(&self) -> &'static str {
                #error_type
            }

            fn wrap(&self) -> String {
                let info = #near_sdk_crate ::serde_json::to_string(self).unwrap_or_default();
                #near_sdk_crate ::wrap_impl(
                    self.error_type(),
                    std::any::type_name::<#ident #ty_generics>(),
                    &info
                )
            }
        }
    };
    if *ident != "BaseError" {
        expanded.extend(quote! {
            impl #impl_generics From<#ident #ty_generics> for String #where_clause {
                fn from(err: #ident #ty_generics) -> Self {
                    #near_sdk_crate ::wrap_error(err)
                }
            }

            impl #impl_generics From<#ident #ty_generics> for #near_sdk_crate ::BaseError #where_clause {
                fn from(err: #ident #ty_generics) -> Self {
                    #near_sdk_crate ::BaseError{
                        error: #near_sdk_crate ::wrap_error(err),
                    }
                }
            }
        });
    }
    TokenStream::from(expanded)
}
