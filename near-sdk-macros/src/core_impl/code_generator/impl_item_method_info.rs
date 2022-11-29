use crate::core_impl::info_extractor::{
    AttrSigInfo, ImplItemMethodInfo, MethodType, SerializerType,
};
use crate::core_impl::utils;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::spanned::Spanned;
use syn::{Meta, Path, ReturnType};

impl ImplItemMethodInfo {
    /// Generate wrapper method for the given method of the contract.
    pub fn method_wrapper(&self) -> TokenStream2 {
        let ImplItemMethodInfo { attr_signature_info, .. } = self;
        // Args provided by `env::input()`.
        let has_input_args = attr_signature_info.input_args().next().is_some();

        let panic_hook = quote! {
            near_sdk::env::setup_panic_hook();
        };
        let arg_struct;
        let arg_parsing;
        if has_input_args {
            arg_struct = attr_signature_info.input_struct_deser();
            let decomposition = attr_signature_info.decomposition_pattern();
            let serializer_invocation = match attr_signature_info.input_serializer {
                SerializerType::JSON => quote! {
                    near_sdk::serde_json::from_slice(
                        &near_sdk::env::input().expect("Expected input since method has arguments.")
                    ).expect("Failed to deserialize input from JSON.")
                },
                SerializerType::Borsh => quote! {
                    near_sdk::borsh::BorshDeserialize::try_from_slice(
                        &near_sdk::env::input().expect("Expected input since method has arguments.")
                    ).expect("Failed to deserialize input from Borsh.")
                },
            };
            arg_parsing = quote! {
                let #decomposition : Input = #serializer_invocation ;
            };
        } else {
            arg_struct = TokenStream2::new();
            arg_parsing = TokenStream2::new();
        };

        let callback_deser = attr_signature_info.callback_deserialization();
        let callback_vec_deser = attr_signature_info.callback_vec_deserialization();

        let AttrSigInfo { non_bindgen_attrs, ident, method_type, is_payable, is_private, .. } =
            attr_signature_info;
        let deposit_check = if *is_payable || matches!(method_type, &MethodType::View) {
            // No check if the method is payable or a view method
            quote! {}
        } else {
            // If method is not payable, do a check to make sure that it doesn't consume deposit
            let error = format!("Method {} doesn't accept deposit", ident);
            quote! {
                if near_sdk::env::attached_deposit() != 0 {
                    near_sdk::env::panic_str(#error);
                }
            }
        };
        let is_private_check = if *is_private {
            let error = format!("Method {} is private", ident);
            quote! {
                if near_sdk::env::current_account_id() != near_sdk::env::predecessor_account_id() {
                    near_sdk::env::panic_str(#error);
                }
            }
        } else {
            quote! {}
        };
        let body = match self.method_body() {
            Ok(wrapper) => wrapper,
            Err(err) => return err.to_compile_error(),
        };
        let non_bindgen_attrs = non_bindgen_attrs.iter().fold(TokenStream2::new(), |acc, value| {
            quote! {
                #acc
                #value
            }
        });
        quote! {
            #non_bindgen_attrs
            #[cfg(target_arch = "wasm32")]
            #[no_mangle]
            pub extern "C" fn #ident() {
                #panic_hook
                #is_private_check
                #deposit_check
                #arg_struct
                #arg_parsing
                #callback_deser
                #callback_vec_deser
                #body
            }
        }
    }

    fn method_body(&self) -> Result<TokenStream2, syn::Error> {
        let ImplItemMethodInfo { attr_signature_info, struct_type, .. } = self;
        let AttrSigInfo { ident, receiver, returns, method_type, is_handles_result, .. } =
            attr_signature_info;
        match method_type {
            MethodType::InitIgnoreState | MethodType::Init => return self.init_method_wrapper(),
            _ => (),
        };
        let arg_list = attr_signature_info.arg_list();
        let is_riff = self.is_riff();
        let contract_deser;
        let method_invocation;
        let contract_ser;
        if let Some(receiver) = receiver {
            let mutability = &receiver.mutability;
            let load = if is_riff {
                quote! {
                    #struct_type::get_lazy()
                }
            } else {
                quote! {
                    near_sdk::env::state_read()
                }
            };
            contract_deser = quote! {
                let #mutability contract: #struct_type = #load.unwrap_or_default();
            };
            method_invocation = quote! {
                contract.#ident(#arg_list)
            };
            if matches!(method_type, &MethodType::Regular) {
                contract_ser = if is_riff {
                    quote! {
                      #struct_type::set_lazy(contract);
                    }
                } else {
                    quote! {
                      near_sdk::env::state_write(&contract);
                    }
                }
            } else {
                contract_ser = TokenStream2::new();
            }
        } else {
            contract_deser = TokenStream2::new();
            method_invocation = quote! {
                #struct_type::#ident(#arg_list)
            };
            contract_ser = TokenStream2::new();
        }
        let res = match returns {
            ReturnType::Default => quote! {
                #contract_deser
                #method_invocation;
                #contract_ser
            },
            ReturnType::Type(_, return_type)
                if utils::type_is_result(return_type) && *is_handles_result =>
            {
                let value_ser = self.result_serializer();
                quote! {
                    #contract_deser
                    let result = #method_invocation;
                    match result {
                        Ok(result) => {
                            #value_ser
                            near_sdk::env::value_return(&result);
                            #contract_ser
                        }
                        Err(err) => near_sdk::FunctionError::panic(&err)
                    }
                }
            }
            ReturnType::Type(_, return_type) if *is_handles_result => {
                return Err(syn::Error::new(
                        return_type.span(),
                        "Method marked with #[handle_result] should return Result<T, E> (where E implements FunctionError).",
                    ));
            }
            ReturnType::Type(_, return_type) if utils::type_is_result(return_type) => {
                return Err(syn::Error::new(
                        return_type.span(),
                        "Serializing Result<T, E> has been deprecated. Consider marking your method \
                        with #[handle_result] if the second generic represents a panicable error or \
                        replacing Result with another two type sum enum otherwise. If you really want \
                        to keep the legacy behavior, mark the method with #[handle_result] and make \
                        it return Result<Result<T, E>, near_sdk::Abort>.",
                    ));
            }
            ReturnType::Type(_, _) => {
                let value_ser = self.result_serializer();
                quote! {
                    #contract_deser
                    let result = #method_invocation;
                    #value_ser
                    near_sdk::env::value_return(&result);
                    #contract_ser
                }
            }
        };
        Ok(res)
    }

    fn init_method_wrapper(&self) -> Result<TokenStream2, syn::Error> {
        let ImplItemMethodInfo { attr_signature_info, struct_type, .. } = self;
        let arg_list = attr_signature_info.arg_list();
        let AttrSigInfo { ident, returns, is_handles_result, .. } = attr_signature_info;
        let is_riff = self.is_riff();
        let state_check = if matches!(&attr_signature_info.method_type, &MethodType::Init) {
            quote! {
                if near_sdk::env::state_exists() {
                    near_sdk::env::panic_str("The contract has already been initialized");
                }
            }
        } else {
            quote! {}
        };
        let state_write = if is_riff {
            quote! {
                #struct_type::set_lazy(contract)
            }
        } else {
            quote! {
                near_sdk::env::state_write(&contract)
            }
        };
        match returns {
            ReturnType::Default => {
                Err(syn::Error::new(ident.span(), "Init methods must return the contract state"))
            }
            ReturnType::Type(_, return_type)
                if utils::type_is_result(return_type) && *is_handles_result =>
            {
                Ok(quote! {
                    #state_check
                    let result = #struct_type::#ident(#arg_list);
                    match result {
                        Ok(contract) => #state_write,
                        Err(err) => near_sdk::FunctionError::panic(&err)
                    }
                })
            }
            ReturnType::Type(_, return_type) if *is_handles_result => Err(syn::Error::new(
                return_type.span(),
                "Method marked with #[handle_result] should return Result<T, E> (where E implements FunctionError).",
            )),
            ReturnType::Type(_, _) => Ok(quote! {
                #state_check
                let contract = #struct_type::#ident(#arg_list);
                #state_write;
            }),
        }
    }

    fn result_serializer(&self) -> TokenStream2 {
        let serialize = match self.attr_signature_info.result_serializer {
            SerializerType::JSON => quote! {
                near_sdk::serde_json::to_vec(&result).expect("Failed to serialize the return value using JSON.")
            },
            SerializerType::Borsh => quote! {
                near_sdk::borsh::BorshSerialize::try_to_vec(&result).expect("Failed to serialize the return value using Borsh.")
            },
        };
        quote! {
            let result = #serialize;
        }
    }

    fn is_riff(&self) -> bool {
        self.impl_attrs.iter().any(|attr| match attr {
            syn::NestedMeta::Meta(Meta::Path(Path { segments, .. })) => {
                segments.iter().any(|path_segment| path_segment.ident == "riff")
            }
            _ => false,
        })
    }
}
