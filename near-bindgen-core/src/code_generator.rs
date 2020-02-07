use syn::export::TokenStream2;
use syn::{FnArg, ImplItemMethod, ReturnType, Token};

use crate::info_extractor::{
    ArgInfo, AttrSignatureInfo, BindgenArgType, ImplMethodInfo, SerializerType,
};
use quote::quote;
use syn::punctuated::Punctuated;

/// Create struct representing input arguments.
/// Each argument is getting converted to a field in a struct. Specifically argument:
/// `ATTRIBUTES ref mut binding @ SUBPATTERN : TYPE` is getting converted to:
/// `binding: SUBTYPE,` where `TYPE` is one of the following: `& SUBTYPE`, `&mut SUBTYPE`, `SUBTYPE`,
/// and `SUBTYPE` is one of the following: `[T; n]`, path like
/// `std::collections::HashMap<SUBTYPE, SUBTYPE>`, or tuple `(SUBTYPE0, SUBTYPE1, ...)`.
/// # Example
/// ```
/// struct Input {
///   arg0: Vec<String>,
///   arg1: [u64; 10],
///   arg2: (u64, Vec<String>),
/// }
/// ```
pub fn input_struct(attr_signature_info: &AttrSignatureInfo) -> TokenStream2 {
    let args: Vec<_> = attr_signature_info.input_args().collect();
    assert!(!args.is_empty(), "Can only generate input struct for when input args are specified");
    let attribute = match &attr_signature_info.input_serializer {
        SerializerType::JSON => quote! {#[derive(serde::Deserialize)]},
        SerializerType::Borsh => quote! {#[derive(borsh::BorshDeserialize)]},
    };
    let mut fields = TokenStream2::new();
    for arg in args {
        let ArgInfo { ty, ident, .. } = &arg;
        fields.extend(quote! {
            #ident: #ty,
        });
    }
    quote! {
        #attribute
        struct Input {
            #fields
        }
    }
}

/// Create pattern that decomposes input struct using correct mutability modifiers.
/// # Example:
/// ```
/// Input {
///     arg0,
///     mut arg1,
///     arg2
/// }
/// ```
pub fn decomposition_pattern(attr_signature_info: &AttrSignatureInfo) -> TokenStream2 {
    let args: Vec<_> = attr_signature_info.input_args().collect();
    assert!(
        !args.is_empty(),
        "Can only generate decomposition pattern for when input args are specified."
    );
    let mut fields = TokenStream2::new();
    for arg in args {
        let ArgInfo { mutability, ident, .. } = &arg;
        fields.extend(quote! {
        #mutability #ident,
        });
    }
    quote! {
        Input {
            #fields
        }
    }
}

/// Create a sequence of arguments that can be used to call the method or the function
/// of the smart contract.
///
/// # Example:
/// ```
/// a, &b, &mut c,
/// ```
pub fn arg_list(attr_signature_info: &AttrSignatureInfo) -> TokenStream2 {
    let mut result = TokenStream2::new();
    for arg in &attr_signature_info.args {
        let ArgInfo { reference, mutability, ident, .. } = &arg;
        result.extend(quote! {
            #reference #mutability #ident,
        });
    }
    result
}

/// Create code that deserializes arguments that were decorated with `#[callback]`
pub fn callback_deserialization(attr_signature_info: &AttrSignatureInfo) -> TokenStream2 {
    attr_signature_info
        .args
        .iter()
        .filter(|arg| match arg.bindgen_ty {
            BindgenArgType::CallbackArg => true,
            _ => false,
        })
        .enumerate()
        .fold(TokenStream2::new(), |acc, (idx, arg)| {
            let ArgInfo {mutability, ident, ty, ..} = arg;
            let read_data = quote! {
                let data: Vec<u8> = match near_bindgen::env::promise_result(#idx) {
                    near_bindgen::PromiseResult::Successful(x) => x,
                    _ => panic!("Callback computation {} was not successful", #idx)
                };
            };
            let invocation = match arg.serializer_ty {
                SerializerType::JSON => quote! {
                    serde_json::from_slice(&data).expect("Failed to deserialize callback using JSON")
                },
                SerializerType::Borsh => quote! {
                    borsh::Deserialize::try_from_slice(&data).expect("Failed to deserialize callback using JSON")
                },
            };
            quote! {
                #acc
                #read_data
                let #mutability #ident: #ty = #invocation;
            }
        })
}

/// Create code that deserializes arguments that were decorated with `#[callback_vec]`.
pub fn callback_vec_deserialization(attr_signature_info: &AttrSignatureInfo) -> TokenStream2 {
    attr_signature_info
        .args
        .iter()
        .filter(|arg| match arg.bindgen_ty {
            BindgenArgType::CallbackArgVec => true,
            _ => false,
        })
        .fold(TokenStream2::new(), |acc, arg| {
            let ArgInfo {mutability, ident, ty, ..} = arg;
            let invocation = match arg.serializer_ty {
                SerializerType::JSON => quote! {
                    serde_json::from_slice(&data).expect("Failed to deserialize callback using JSON")
                },
                SerializerType::Borsh => quote! {
                    borsh::Deserialize::try_from_slice(&data).expect("Failed to deserialize callback using JSON")
                },
            };
            quote! {
                #acc
                let #mutability #ident: #ty = (0..near_bindgen::env::promise_results_count())
                .map(|i| {
                    let data: Vec<u8> = match near_bindgen::env::promise_result(i) {
                        near_bindgen::PromiseResult::Successful(x) => x,
                        _ => panic!("Callback computation {} was not successful", i)
                    };
                    #invocation
                }).collect();
            }
        })
}

/// Generate wrapper method for the given method of the contract.
pub fn method_wrapper(method_info: &ImplMethodInfo) -> TokenStream2 {
    let ImplMethodInfo { attr_signature_info, struct_type, .. } = method_info;
    // Args provided by `env::input()`.
    let has_input_args = attr_signature_info.input_args().next().is_some();

    let env_creation = quote! {
        near_bindgen::env::set_blockchain_interface(Box::new(near_blockchain::NearBlockchain {}));
    };
    let arg_struct;
    let arg_parsing;
    if has_input_args {
        arg_struct = input_struct(attr_signature_info);
        let decomposition = decomposition_pattern(attr_signature_info);
        let serializer_invocation = match attr_signature_info.input_serializer {
            SerializerType::JSON => quote! {
            serde_json::from_slice(
                &near_bindgen::env::input().expect("Expected input since method has arguments.")
            ).expect("Failed to deserialize input from JSON.")
            },
            SerializerType::Borsh => quote! {
            borsh::Deserialize::try_from_slice(
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

    let callback_deser = callback_deserialization(attr_signature_info);
    let callback_vec_deser = callback_vec_deserialization(attr_signature_info);

    let arg_list = arg_list(attr_signature_info);
    let AttrSignatureInfo {
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
                        let result = borsh::BorshSerialize::try_to_vec(&contract, &result).expect("Failed to serialize the return value using Borsh.");
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
            #env_creation
            #arg_struct
            #arg_parsing
            #callback_deser
            #callback_vec_deser
            #body
        }
    }
}

/// Original method from `impl` section with adjusted attributes.
pub fn processed_impl_method(method_info: ImplMethodInfo) -> ImplItemMethod {
    let ImplMethodInfo { mut original, attr_signature_info, .. } = method_info;
    let AttrSignatureInfo { receiver, args, non_bindgen_attrs, .. } = attr_signature_info;
    original.attrs = non_bindgen_attrs;
    let mut inputs: Punctuated<FnArg, Token![,]> = Default::default();
    if let Some(receiver) = receiver {
        inputs.push(FnArg::Receiver(receiver));
    }
    for arg_info in args {
        let ArgInfo { mut original, non_bindgen_attrs, .. } = arg_info;
        original.attrs = non_bindgen_attrs;
        inputs.push(FnArg::Typed(original));
    }
    original.sig.inputs = inputs;
    original
}
