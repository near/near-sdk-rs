use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::{format_ident, quote};
use syn::spanned::Spanned;
use syn::{Attribute, Lit::Str, Meta::NameValue, MetaNameValue, ReturnType, Type};

use crate::core_impl::{
    utils, AttrSigInfo, BindgenArgType, ImplItemMethodInfo, ItemImplInfo, MethodType,
    SerializerType,
};

pub fn generate(i: &ItemImplInfo) -> TokenStream2 {
    let public_functions: Vec<&ImplItemMethodInfo> =
        i.methods.iter().filter(|m| m.is_public || i.is_trait_impl).collect();
    if public_functions.is_empty() {
        // Short-circuit if there are no public functions to export to ABI
        return TokenStream2::new();
    }

    let functions: Vec<TokenStream2> = public_functions.iter().map(|m| m.abi_struct()).collect();
    let first_function_name = &public_functions[0].attr_signature_info.ident;
    let near_abi_symbol = format_ident!("__near_abi_{}", first_function_name);
    quote! {
        #[cfg(not(target_arch = "wasm32"))]
        const _: () = {
            #[no_mangle]
            pub fn #near_abi_symbol() -> near_sdk::__private::ChunkedAbiEntry {
                let mut gen = near_sdk::__private::schemars::gen::SchemaGenerator::default();
                let functions = vec![#(#functions),*];
                near_sdk::__private::ChunkedAbiEntry::new(
                    functions,
                    gen.into_root_schema_for::<String>()
                )
            }
        };
    }
}

impl ImplItemMethodInfo {
    /// Generates ABI struct for this function.
    ///
    /// # Example:
    /// The following function:
    /// ```ignore
    /// /// I am a function.
    /// pub fn f3(&mut self, arg0: FancyStruct, arg1: u64) -> Result<IsOk, Error> { }
    /// ```
    /// will produce this struct:
    /// ```ignore
    /// near_abi::AbiFunction {
    ///     name: "f3".to_string(),
    ///     doc: Some(" I am a function.".to_string()),
    ///     is_view: false,
    ///     is_init: false,
    ///     is_payable: false,
    ///     is_private: false,
    ///     params: vec![
    ///         near_abi::AbiParameter {
    ///             name: "arg0".to_string(),
    ///             typ: near_abi::AbiType::Json {
    ///                 type_schema: gen.subschema_for::<FancyStruct>(),
    ///             },
    ///         },
    ///         near_abi::AbiParameter {
    ///             name: "arg1".to_string(),
    ///             typ: near_abi::AbiType::Json {
    ///                 type_schema: gen.subschema_for::<u64>(),
    ///             },
    ///         }
    ///     ],
    ///     callbacks: vec![],
    ///     callbacks_vec: None,
    ///     result: near_abi::AbiType::Json {
    ///         type_schema: gen.subschema_for::<IsOk>(),
    ///     }
    /// }
    /// ```
    /// If args are serialized with Borsh it will not include `#[derive(borsh::BorshSchema)]`.
    pub fn abi_struct(&self) -> TokenStream2 {
        let function_name_str = self.attr_signature_info.ident.to_string();
        let function_doc = match parse_rustdoc(&self.attr_signature_info.non_bindgen_attrs) {
            Some(doc) => quote! { Some(#doc.to_string()) },
            None => quote! { None },
        };
        let is_view = matches!(&self.attr_signature_info.method_type, &MethodType::View);
        let is_init = matches!(
            &self.attr_signature_info.method_type,
            &MethodType::Init | &MethodType::InitIgnoreState
        );
        let AttrSigInfo { is_payable, is_private, is_handles_result, .. } =
            self.attr_signature_info;

        let mut params = Vec::<TokenStream2>::new();
        let mut callbacks = Vec::<TokenStream2>::new();
        let mut callback_vec: Option<TokenStream2> = None;
        for arg in &self.attr_signature_info.args {
            let typ = &arg.ty;
            let arg_name = arg.ident.to_string();
            match arg.bindgen_ty {
                BindgenArgType::Regular => {
                    let abi_type = generate_abi_type(typ, &arg.serializer_ty);
                    params.push(quote! {
                        near_sdk::__private::AbiParameter {
                            name: #arg_name.to_string(),
                            typ: #abi_type
                        }
                    });
                }
                BindgenArgType::CallbackArg => {
                    callbacks.push(generate_abi_type(typ, &arg.serializer_ty));
                }
                BindgenArgType::CallbackResultArg => {
                    let typ = if let Some(ok_type) = utils::extract_ok_type(typ) {
                        ok_type
                    } else {
                        return syn::Error::new_spanned(
                            &arg.ty,
                            "Function parameters marked with \
                                #[callback_result] should have type Result<T, PromiseError>",
                        )
                        .into_compile_error();
                    };
                    callbacks.push(generate_abi_type(typ, &arg.serializer_ty));
                }
                BindgenArgType::CallbackArgVec => {
                    if callback_vec.is_none() {
                        let typ = if let Some(vec_type) = utils::extract_vec_type(typ) {
                            vec_type
                        } else {
                            return syn::Error::new_spanned(
                                &arg.ty,
                                "Function parameters marked with  #[callback_vec] should have type Vec<T>",
                            )
                            .into_compile_error();
                        };

                        let abi_type =
                            generate_abi_type(typ, &self.attr_signature_info.result_serializer);
                        callback_vec = Some(quote! { Some(#abi_type) })
                    } else {
                        return syn::Error::new(
                            Span::call_site(),
                            "A function can only have one #[callback_vec] parameter.",
                        )
                        .to_compile_error();
                    }
                }
            };
        }
        let callback_vec = callback_vec.unwrap_or(quote! { None });

        let result = match self.attr_signature_info.method_type {
            MethodType::Init | MethodType::InitIgnoreState => {
                // Init methods must return the contract state, so the return type does not matter
                quote! {
                    None
                }
            }
            _ => match &self.attr_signature_info.returns {
                ReturnType::Default => {
                    quote! {
                        None
                    }
                }
                ReturnType::Type(_, ty) if is_handles_result && utils::type_is_result(ty) => {
                    let ty = if let Some(ty) = utils::extract_ok_type(ty) {
                        ty
                    } else {
                        return syn::Error::new_spanned(
                            ty,
                            "Function marked with #[handle_result] should have return type Result<T, E> (where E implements FunctionError).",
                        )
                        .into_compile_error();
                    };
                    let abi_type =
                        generate_abi_type(ty, &self.attr_signature_info.result_serializer);
                    quote! { Some(#abi_type) }
                }
                ReturnType::Type(_, ty) if is_handles_result => {
                    return syn::Error::new(
                        ty.span(),
                        "Method marked with #[handle_result] should return Result<T, E> (where E implements FunctionError).",
                    )
                    .to_compile_error();
                }
                ReturnType::Type(_, ty) => {
                    let abi_type =
                        generate_abi_type(ty, &self.attr_signature_info.result_serializer);
                    quote! { Some(#abi_type) }
                }
            },
        };

        quote! {
             near_sdk::__private::AbiFunction {
                 name: #function_name_str.to_string(),
                 doc: #function_doc,
                 is_view: #is_view,
                 is_init: #is_init,
                 is_payable: #is_payable,
                 is_private: #is_private,
                 params: vec![#(#params),*],
                 callbacks: vec![#(#callbacks),*],
                 callbacks_vec: #callback_vec,
                 result: #result
             }
        }
    }
}

fn generate_abi_type(ty: &Type, serializer_type: &SerializerType) -> TokenStream2 {
    match serializer_type {
        SerializerType::JSON => quote! {
            near_sdk::__private::AbiType::Json {
                type_schema: gen.subschema_for::<#ty>(),
            }
        },
        SerializerType::Borsh => quote! {
            near_sdk::__private::AbiType::Borsh {
                type_schema: <#ty>::schema_container(),
            }
        },
    }
}

pub fn parse_rustdoc(attrs: &[Attribute]) -> Option<String> {
    let doc = attrs
        .iter()
        .filter_map(|attr| {
            if attr.path.is_ident("doc") {
                if let NameValue(MetaNameValue { lit: Str(s), .. }) = attr.parse_meta().ok()? {
                    Some(s.value())
                } else {
                    None
                }
            } else {
                None
            }
        })
        .collect::<Vec<_>>()
        .join("\n");

    if doc.is_empty() {
        None
    } else {
        Some(doc)
    }
}
