use crate::core_impl::{utils, AttrSigInfo};
use crate::core_impl::{BindgenArgType, SerializerType};
use crate::{ImplItemMethodInfo, MethodType};
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::quote;
use syn::spanned::Spanned;
use syn::{ReturnType, Type};

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
                type_schema: #ty::schema_container(),
            }
        },
    }
}
