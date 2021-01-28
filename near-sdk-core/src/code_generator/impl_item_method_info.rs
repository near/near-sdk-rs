use crate::info_extractor::{AttrSigInfo, ImplItemMethodInfo, InputStructType, SerializerType};
use quote::quote;
use syn::export::TokenStream2;
use syn::{ReturnType, Signature};

impl ImplItemMethodInfo {
    /// Generate wrapper method for the given method of the contract.
    pub fn method_wrapper(&self) -> TokenStream2 {
        let ImplItemMethodInfo { attr_signature_info, struct_type, .. } = self;
        // Args provided by `env::input()`.
        let has_input_args = attr_signature_info.input_args().next().is_some();

        let panic_hook = quote! {
            near_sdk::env::setup_panic_hook();
        };
        let env_creation = quote! {
            near_sdk::env::set_blockchain_interface(Box::new(near_blockchain::NearBlockchain {}));
        };
        let arg_struct;
        let arg_parsing;
        if has_input_args {
            arg_struct = attr_signature_info.input_struct(InputStructType::Deserialization);
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

        let arg_list = attr_signature_info.arg_list();
        let AttrSigInfo {
            non_bindgen_attrs,
            ident,
            receiver,
            returns,
            result_serializer,
            is_init,
            is_payable,
            is_private,
            is_view,
            ..
        } = attr_signature_info;
        let deposit_check = if *is_payable || *is_view {
            // No check if the method is payable or a view method
            quote! {}
        } else {
            // If method is not payable, do a check to make sure that it doesn't consume deposit
            let error = format!("Method {} doesn't accept deposit", ident.to_string());
            quote! {
                if near_sdk::env::attached_deposit() != 0 {
                    near_sdk::env::panic(#error.as_bytes());
                }
            }
        };
        let is_private_check = if *is_private {
            let error = format!("Method {} is private", ident.to_string());
            quote! {
                if env::current_account_id() != env::predecessor_account_id() {
                    near_sdk::env::panic(#error.as_bytes());
                }
            }
        } else {
            quote! {}
        };
        let body = if *is_init {
            quote! {
                let contract = #struct_type::#ident(#arg_list);
                near_sdk::env::state_write(&contract);
            }
        } else {
            let contract_deser;
            let method_invocation;
            let contract_ser;
            if let Some(receiver) = receiver {
                let mutability = &receiver.mutability;
                contract_deser = quote! {
                    let #mutability contract: #struct_type = near_sdk::env::state_read().unwrap_or_default();
                };
                method_invocation = quote! {
                    contract.#ident(#arg_list)
                };
                if !is_view {
                    contract_ser = quote! {
                        near_sdk::env::state_write(&contract);
                    };
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
            match returns {
                ReturnType::Default => quote! {
                    #contract_deser
                    #method_invocation;
                    #contract_ser
                },
                ReturnType::Type(_, _) => {
                    let value_ser = match result_serializer {
                        SerializerType::JSON => quote! {
                            let result = near_sdk::serde_json::to_vec(&result).expect("Failed to serialize the return value using JSON.");
                        },
                        SerializerType::Borsh => quote! {
                            let result = near_sdk::borsh::BorshSerialize::try_to_vec(&result).expect("Failed to serialize the return value using Borsh.");
                        },
                    };
                    quote! {
                    #contract_deser
                    let result = #method_invocation;
                    #value_ser
                    near_sdk::env::value_return(&result);
                    #contract_ser
                    }
                }
            }
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
                #env_creation
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

    pub fn marshal_method(&self) -> TokenStream2 {
        let ImplItemMethodInfo { attr_signature_info, .. } = self;
        let has_input_args = attr_signature_info.input_args().next().is_some();

        let pat_type_list = attr_signature_info.pat_type_list();
        let json_args = if has_input_args {
            let args: TokenStream2 = attr_signature_info
                .input_args()
                .fold(None, |acc: Option<TokenStream2>, value| {
                    let ident = &value.ident;
                    let ident_str = format!("{}", ident.to_string());
                    Some(match acc {
                        None => quote! { #ident_str: #ident },
                        Some(a) => quote! { #a, #ident_str: #ident },
                    })
                })
                .unwrap();
            quote! {
              let args = near_sdk::serde_json::json!({#args});
            }
        } else {
            quote! {
             let args = near_sdk::serde_json::json!({});
            }
        };

        let AttrSigInfo {
            non_bindgen_attrs,
            ident,
            // receiver,
            // returns,
            // result_serializer,
            // is_init,
            is_view,
            original_sig,
            ..
        } = attr_signature_info;
        let return_ident = quote! { -> near_sdk::PendingContractTx };
        let params = quote! {
            &self, #pat_type_list
        };
        let ident_str = format!("{}", ident.to_string());
        let body = if *is_view {
            quote! {
                near_sdk::PendingContractTx::new(&self.account_id, #ident_str, args, true)
            }
        } else {
            quote! {
                near_sdk::PendingContractTx::new(&self.account_id, #ident_str, args, false)
            }
        };
        let non_bindgen_attrs = non_bindgen_attrs.iter().fold(TokenStream2::new(), |acc, value| {
            quote! {
                #acc
                #value
            }
        });
        let Signature { generics, .. } = original_sig;
        quote! {
            #[cfg(not(target_arch = "wasm32"))]
            #non_bindgen_attrs
            pub fn #ident#generics(#params) #return_ident {
                #json_args
                #body
            }
        }
    }
}
