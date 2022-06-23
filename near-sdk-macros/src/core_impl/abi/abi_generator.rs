use crate::core_impl::utils;
use crate::core_impl::{BindgenArgType, SerializerType};
use crate::{ImplItemMethodInfo, MethodType};
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::quote;
use syn::ReturnType;

impl ImplItemMethodInfo {
    /// Generates ABI struct for this function.
    ///
    /// # Example:
    /// The following function:
    /// ```ignore
    /// fn f3(&mut self, arg0: FancyStruct, arg1: u64) -> Result<IsOk, Error> { }
    /// ```
    /// will produce this struct:
    /// ```ignore
    /// near_sdk::__private::AbiFunction {
    ///     name: "f3".to_string(),
    ///     is_view: false,
    ///     is_init: false,
    ///     params: vec![
    ///         near_sdk::__private::AbiParameter {
    ///             type_id: 0,
    ///             serialization_type: "json",
    ///         },
    ///         near_sdk::__private::AbiParameter {
    ///             type_id: 1,
    ///             serialization_type: "json",
    ///         }
    ///     ],
    ///     callbacks: vec![],
    ///     callbacks_vec: None,
    ///     result: near_sdk::__private::AbiParameter {
    ///         type_id: 2,
    ///         serialization_type: "json",
    ///     }
    /// }
    /// ```
    /// If args are serialized with Borsh it will not include `#[derive(borsh::BorshSchema)]`.
    pub fn abi_struct(&self) -> TokenStream2 {
        let function_name_str = self.attr_signature_info.ident.to_string();
        let is_view = matches!(&self.attr_signature_info.method_type, &MethodType::View);
        let is_init = matches!(
            &self.attr_signature_info.method_type,
            &MethodType::Init | &MethodType::InitIgnoreState
        );
        let is_payable = self.attr_signature_info.is_payable;
        let is_private = self.attr_signature_info.is_private;

        let mut params = Vec::<TokenStream2>::new();
        let mut callbacks = Vec::<TokenStream2>::new();
        let mut callback_vec: Option<TokenStream2> = None;
        for arg in &self.attr_signature_info.args {
            let typ = &arg.ty;
            let serialization_type = abi_serialization_type(&arg.serializer_ty);
            let arg_name = arg.ident.to_string();
            match arg.bindgen_ty {
                BindgenArgType::Regular => {
                    params.push(quote! {
                        near_sdk::__private::AbiParameter {
                            name: #arg_name.to_string(),
                            type_schema: gen.subschema_for::<#typ>(),
                            serialization_type: #serialization_type,
                        }
                    });
                }
                BindgenArgType::CallbackArg => {
                    callbacks.push(quote! {
                        near_sdk::__private::AbiType {
                            type_schema: gen.subschema_for::<#typ>(),
                            serialization_type: #serialization_type,
                        }
                    });
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
                    callbacks.push(quote! {
                        near_sdk::__private::AbiType {
                            type_schema: gen.subschema_for::<#typ>(),
                            serialization_type: #serialization_type,
                        }
                    });
                }
                BindgenArgType::CallbackArgVec => {
                    if callback_vec.is_none() {
                        callback_vec = Some(quote! {
                            Some(
                                near_sdk::__private::AbiType {
                                    type_schema: gen.subschema_for::<#typ>(),
                                    serialization_type: #serialization_type,
                                }
                            )
                        })
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
                ReturnType::Type(_, ty) => {
                    let serialization_type =
                        abi_serialization_type(&self.attr_signature_info.result_serializer);
                    quote! {
                        Some(
                            near_sdk::__private::AbiType {
                                type_schema: gen.subschema_for::<#ty>(),
                                serialization_type: #serialization_type,
                            }
                        )
                    }
                }
            },
        };

        quote! {
             near_sdk::__private::AbiFunction {
                 name: #function_name_str.to_string(),
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

fn abi_serialization_type(serializer_type: &SerializerType) -> TokenStream2 {
    match serializer_type {
        SerializerType::JSON => quote! {
            near_sdk::__private::AbiSerializationType::Json
        },
        SerializerType::Borsh => quote! {
            near_sdk::__private::AbiSerializationType::Borsh
        },
    }
}
