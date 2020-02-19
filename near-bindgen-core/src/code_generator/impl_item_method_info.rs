use crate::info_extractor::{AttrSigInfo, ImplItemMethodInfo, SerializerType};
use quote::quote;
use syn::export::TokenStream2;
use syn::ReturnType;

impl ImplItemMethodInfo {
    /// Generate wrapper method for the given method of the contract.
    pub fn method_wrapper(&self) -> TokenStream2 {
        let ImplItemMethodInfo { attr_signature_info, struct_type, .. } = self;
        // Args provided by `env::input()`.
        let has_input_args = attr_signature_info.input_args().next().is_some();

        let panic_hook = quote! {
            near_bindgen::env::setup_panic_hook();
        };
        let env_creation = quote! {
            near_bindgen::env::set_blockchain_interface(Box::new(near_blockchain::NearBlockchain {}));
        };
        let arg_struct;
        let arg_parsing;
        if has_input_args {
            arg_struct = attr_signature_info.input_struct();
            let decomposition = attr_signature_info.decomposition_pattern();
            let serializer_invocation = match attr_signature_info.input_serializer {
                SerializerType::JSON => quote! {
                serde_json::from_slice(
                    &near_bindgen::env::input().expect("Expected input since method has arguments.")
                ).expect("Failed to deserialize input from JSON.")
                },
                SerializerType::Borsh => quote! {
                borsh::BorshDeserialize::try_from_slice(
                    &near_bindgen::env::input().expect("Expected input since method has arguments.")
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
            ..
        } = attr_signature_info;
        let body = if *is_init {
            quote! {
                let contract = #struct_type::#ident(#arg_list);
                near_bindgen::env::state_write(&contract);
            }
        } else {
            let contract_deser;
            let method_invocation;
            let contract_ser;
            if let Some(receiver) = receiver {
                let mutability = &receiver.mutability;
                let reference = &receiver.reference;
                contract_deser = quote! {
                    let #mutability contract: #struct_type = near_bindgen::env::state_read().unwrap_or_default();
                };
                method_invocation = quote! {
                    contract.#ident(#arg_list)
                };
                if mutability.is_some() && reference.is_some() {
                    contract_ser = quote! {
                        near_bindgen::env::state_write(&contract);
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
                            let result = serde_json::to_vec(&result).expect("Failed to serialize the return value using JSON.");
                        },
                        SerializerType::Borsh => quote! {
                            let result = borsh::BorshSerialize::try_to_vec(&result).expect("Failed to serialize the return value using Borsh.");
                        },
                    };
                    quote! {
                    #contract_deser
                    let result = #method_invocation;
                    #value_ser
                    near_bindgen::env::value_return(&result);
                    #contract_ser
                    }
                }
            }
        };
        let non_bindgen_attrs =
            non_bindgen_attrs.into_iter().fold(TokenStream2::new(), |acc, value| {
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
                #arg_struct
                #arg_parsing
                #callback_deser
                #callback_vec_deser
                #body
            }
        }
    }
}
