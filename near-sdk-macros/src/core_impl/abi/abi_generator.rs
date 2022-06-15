use crate::{
    core_impl::{BindgenArgType, SerializerType},
    ImplItemMethodInfo, MethodType,
};

use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::quote;
use syn::ReturnType;

use super::TypeRegistry;

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
    pub fn abi_struct(&self, registry: &mut TypeRegistry) -> TokenStream2 {
        let function_name_str = self.attr_signature_info.ident.to_string();
        let is_view = matches!(&self.attr_signature_info.method_type, &MethodType::View);
        let is_init = matches!(
            &self.attr_signature_info.method_type,
            &MethodType::Init | &MethodType::InitIgnoreState
        );
        let params: Vec<TokenStream2> = self
            .attr_signature_info
            .input_args()
            .map(|arg| {
                let type_id = registry.register_type(Box::new(arg.ty.clone()));
                let serialization_type = abi_serializer_type(&arg.serializer_ty);
                let arg_name = arg.ident.to_string();
                quote! {
                    near_sdk::__private::AbiParameter {
                        name: #arg_name.to_string(),
                        type_id: #type_id,
                        serialization_type: #serialization_type,
                    }
                }
            })
            .collect();
        let callbacks: Vec<TokenStream2> = self
            .attr_signature_info
            .args
            .iter()
            .filter(|arg| {
                matches!(arg.bindgen_ty, BindgenArgType::CallbackArg)
                    || matches!(arg.bindgen_ty, BindgenArgType::CallbackResultArg)
            })
            .map(|arg| {
                let type_id = registry.register_type(Box::new(arg.ty.clone()));
                let serialization_type = abi_serializer_type(&arg.serializer_ty);
                quote! {
                    near_sdk::__private::AbiType {
                        type_id: #type_id,
                        serialization_type: #serialization_type,
                    }
                }
            })
            .collect();
        let callback_vec = self
            .attr_signature_info
            .args
            .iter()
            .filter(|arg| matches!(arg.bindgen_ty, BindgenArgType::CallbackArgVec))
            .collect::<Vec<_>>();
        if callback_vec.len() > 1 {
            return syn::Error::new(
                Span::call_site(),
                "A function can only have one #[callback_vec] parameter.",
            )
            .to_compile_error();
        }
        let callback_vec = match callback_vec.last() {
            Some(arg) => {
                let type_id = registry.register_type(Box::new(arg.ty.clone()));
                let serialization_type = abi_serializer_type(&arg.serializer_ty);
                quote! {
                    Some(
                        near_sdk::__private::AbiType {
                            type_id: #type_id,
                            serialization_type: #serialization_type,
                        }
                    )
                }
            }
            None => {
                quote! { None }
            }
        };
        let result = match &self.attr_signature_info.returns {
            ReturnType::Default => {
                quote! {
                    None
                }
            }
            ReturnType::Type(_, ty) => {
                let type_id = registry.register_type(ty.clone());
                let serialization_type =
                    abi_serializer_type(&self.attr_signature_info.result_serializer);
                quote! {
                    Some(
                        near_sdk::__private::AbiType {
                            type_id: #type_id,
                            serialization_type: #serialization_type,
                        }
                    )
                }
            }
        };

        quote! {
             near_sdk::__private::AbiFunction {
                 name: #function_name_str.to_string(),
                 is_view: #is_view,
                 is_init: #is_init,
                 params: vec![#(#params),*],
                 callbacks: vec![#(#callbacks),*],
                 callbacks_vec: #callback_vec,
                 result: #result
             }
        }
    }
}

fn abi_serializer_type(serializer_type: &SerializerType) -> TokenStream2 {
    match serializer_type {
        SerializerType::JSON => quote! {
            near_sdk::__private::AbiSerializerType::Json
        },
        SerializerType::Borsh => quote! {
            near_sdk::__private::AbiSerializerType::Borsh
        },
    }
}
