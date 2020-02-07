#![recursion_limit = "128"]
use crate::code_generator::{method_wrapper, processed_impl_method};
use crate::info_extractor::ImplMethodInfo;
use syn::export::TokenStream2;
use syn::spanned::Spanned;
use syn::{Error, ImplItem, ItemImpl};

mod code_generator;
mod info_extractor;

/// Processes `impl` section of the struct.
/// # Args:
/// `item_impl` -- tokens representing `impl .. {}` body;
pub fn process_impl(item_impl: &mut ItemImpl) -> TokenStream2 {
    if !item_impl.generics.params.is_empty() {
        return Error::new(
            item_impl.generics.params.span(),
            "Impl type parameters are not supported for smart contracts.",
        )
        .to_compile_error();
    }
    let mut output = TokenStream2::new();
    let is_trait_impl = item_impl.trait_.is_some();

    // Type for which impl is called.
    let impl_type = item_impl.self_ty.as_ref();
    for subitem in item_impl.items.iter_mut() {
        if let ImplItem::Method(m) = subitem {
            let method_info =
                ImplMethodInfo::new((*m).clone(), (*impl_type).clone(), is_trait_impl);
            let method_info = match method_info {
                Ok(x) => x,
                Err(err) => {
                    return err.to_compile_error();
                }
            };

            if method_info.is_public {
                output.extend(method_wrapper(&method_info));
                *m = processed_impl_method(method_info);
            }
        }
    }
    output
}

// Rustfmt removes comas.
#[rustfmt::skip]
#[cfg(test)]
mod tests {
    use syn::{Type, ImplItemMethod, parse_quote};
    use crate::info_extractor::ImplMethodInfo;
    use crate::code_generator::method_wrapper;
    use quote::quote;


    #[test]
    fn trait_implt() {
        let impl_type: Type = syn::parse_str("Hello").unwrap();
        let method: ImplItemMethod = syn::parse_str("fn method(&self) { }").unwrap();
        let method_info = ImplMethodInfo::new(method, impl_type, false).unwrap();
        let actual = method_wrapper(&method_info);
        let expected = quote!(
            #[cfg(target_arch = "wasm32")]
            #[no_mangle]
            pub extern "C" fn method() {
                near_bindgen::env::set_blockchain_interface(Box::new(near_blockchain::NearBlockchain {}));
                let contract: Hello = near_bindgen::env::state_read().unwrap_or_default();
                contract.method();
            }
        );
        assert_eq!(expected.to_string(), actual.to_string());
    }

    #[test]
    fn no_args_no_return_no_mut() {
        let impl_type: Type = syn::parse_str("Hello").unwrap();
        let method: ImplItemMethod = syn::parse_str("pub fn method(&self) { }").unwrap();
        let method_info = ImplMethodInfo::new(method, impl_type, false).unwrap();
        let actual = method_wrapper(&method_info);
        let expected = quote!(
            #[cfg(target_arch = "wasm32")]
            #[no_mangle]
            pub extern "C" fn method() {
                near_bindgen::env::set_blockchain_interface(Box::new(near_blockchain::NearBlockchain {}));
                let contract: Hello = near_bindgen::env::state_read().unwrap_or_default();
                contract.method();
            }
        );
        assert_eq!(expected.to_string(), actual.to_string());
    }

    #[test]
    fn no_args_no_return_mut() {
        let impl_type: Type = syn::parse_str("Hello").unwrap();
        let method: ImplItemMethod = syn::parse_str("pub fn method(&mut self) { }").unwrap();
        let method_info = ImplMethodInfo::new(method, impl_type, false).unwrap();
        let actual = method_wrapper(&method_info);
        let expected = quote!(
            #[cfg(target_arch = "wasm32")]
            #[no_mangle]
            pub extern "C" fn method() {
                near_bindgen::env::set_blockchain_interface(Box::new(near_blockchain::NearBlockchain {}));
                let mut contract: Hello = near_bindgen::env::state_read().unwrap_or_default();
                contract.method();
                near_bindgen::env::state_write(&contract);
            }
        );
        assert_eq!(expected.to_string(), actual.to_string());
    }

    #[test]
    fn arg_no_return_no_mut() {
        let impl_type: Type = syn::parse_str("Hello").unwrap();
        let method: ImplItemMethod = syn::parse_str("pub fn method(&self, k: u64) { }").unwrap();
        let method_info = ImplMethodInfo::new(method, impl_type, false).unwrap();
        let actual = method_wrapper(&method_info);
        let expected = quote!(
            #[cfg(target_arch = "wasm32")]
            #[no_mangle]
            pub extern "C" fn method() {
                near_bindgen::env::set_blockchain_interface(Box::new(near_blockchain::NearBlockchain {}));
                #[derive(serde::Deserialize)]
                struct Input {
                    k: u64,
                }
                let Input { k, }: Input = serde_json::from_slice(
                    &near_bindgen::env::input().expect("Expected input since method has arguments.")
                )
                .expect("Failed to deserialize input from JSON.");
                let contract: Hello = near_bindgen::env::state_read().unwrap_or_default();
                contract.method(k, );
            }
        );
        assert_eq!(expected.to_string(), actual.to_string());
    }



    #[test]
    fn args_no_return_mut() {
        let impl_type: Type = syn::parse_str("Hello").unwrap();
        let method: ImplItemMethod =
            syn::parse_str("pub fn method(&mut self, k: u64, m: Bar) { }").unwrap();
        let method_info = ImplMethodInfo::new(method, impl_type, false).unwrap();
        let actual = method_wrapper(&method_info);
        let expected = quote!(
                #[cfg(target_arch = "wasm32")]
                #[no_mangle]
                pub extern "C" fn method() {
                    near_bindgen::env::set_blockchain_interface(Box::new(near_blockchain::NearBlockchain {}));
                    #[derive(serde::Deserialize)]
                    struct Input {
                        k: u64,
                        m: Bar,
                    }
                    let Input { k, m, }: Input = serde_json::from_slice(
                        &near_bindgen::env::input().expect("Expected input since method has arguments.")
                    )
                    .expect("Failed to deserialize input from JSON.");
                    let mut contract: Hello = near_bindgen::env::state_read().unwrap_or_default();
                    contract.method(k, m, );
                    near_bindgen::env::state_write(&contract);
                }
        );
        assert_eq!(expected.to_string(), actual.to_string());
    }

    #[test]
    fn args_return_mut() {
        let impl_type: Type = syn::parse_str("Hello").unwrap();
        let method: ImplItemMethod =
            syn::parse_str("pub fn method(&mut self, k: u64, m: Bar) -> Option<u64> { }").unwrap();
        let method_info = ImplMethodInfo::new(method, impl_type, false).unwrap();
        let actual = method_wrapper(&method_info);
        let expected = quote!(
                #[cfg(target_arch = "wasm32")]
                #[no_mangle]
                pub extern "C" fn method() {
                    near_bindgen::env::set_blockchain_interface(Box::new(near_blockchain::NearBlockchain {}));
                    #[derive(serde::Deserialize)]
                    struct Input {
                        k: u64,
                        m: Bar,
                    }
                    let Input { k, m, }: Input = serde_json::from_slice(
                        &near_bindgen::env::input().expect("Expected input since method has arguments.")
                    )
                    .expect("Failed to deserialize input from JSON.");
                    let mut contract: Hello = near_bindgen::env::state_read().unwrap_or_default();
                    let result = contract.method(k, m, );
                    let result =
                        serde_json::to_vec(&result).expect("Failed to serialize the return value using JSON.");
                    near_bindgen::env::value_return(&result);
                    near_bindgen::env::state_write(&contract);
                }
        );
        assert_eq!(expected.to_string(), actual.to_string());
    }

    #[test]
    fn args_return_ref() {
        let impl_type: Type = syn::parse_str("Hello").unwrap();
        let method: ImplItemMethod =
            syn::parse_str("pub fn method(&self) -> &Option<u64> { }").unwrap();
        let method_info = ImplMethodInfo::new(method, impl_type, false).unwrap();
        let actual = method_wrapper(&method_info);
        let expected = quote!(
            #[cfg(target_arch = "wasm32")]
            #[no_mangle]
            pub extern "C" fn method() {
                near_bindgen::env::set_blockchain_interface(Box::new(near_blockchain::NearBlockchain {}));
                let contract: Hello = near_bindgen::env::state_read().unwrap_or_default();
                let result = contract.method();
                let result =
                    serde_json::to_vec(&result).expect("Failed to serialize the return value using JSON.");
                near_bindgen::env::value_return(&result);
            }
        );
        assert_eq!(expected.to_string(), actual.to_string());
    }

    #[test]
    fn arg_ref() {
        let impl_type: Type = syn::parse_str("Hello").unwrap();
        let method: ImplItemMethod = syn::parse_str("pub fn method(&self, k: &u64) { }").unwrap();
        let method_info = ImplMethodInfo::new(method, impl_type, false).unwrap();
        let actual = method_wrapper(&method_info);
        let expected = quote!(
                #[cfg(target_arch = "wasm32")]
                #[no_mangle]
                pub extern "C" fn method() {
                    near_bindgen::env::set_blockchain_interface(Box::new(near_blockchain::NearBlockchain {}));
                    #[derive(serde::Deserialize)]
                    struct Input {
                        k: u64,
                    }
                    let Input { k, }: Input = serde_json::from_slice(
                        &near_bindgen::env::input().expect("Expected input since method has arguments.")
                    )
                    .expect("Failed to deserialize input from JSON.");
                    let contract: Hello = near_bindgen::env::state_read().unwrap_or_default();
                    contract.method(&k, );
                }
        );
        assert_eq!(expected.to_string(), actual.to_string());
    }

    #[test]
    fn arg_mut_ref() {
        let impl_type: Type = syn::parse_str("Hello").unwrap();
        let method: ImplItemMethod =
            syn::parse_str("pub fn method(&self, k: &mut u64) { }").unwrap();
        let method_info = ImplMethodInfo::new(method, impl_type, false).unwrap();
        let actual = method_wrapper(&method_info);
        let expected = quote!(
            #[cfg(target_arch = "wasm32")]
            #[no_mangle]
            pub extern "C" fn method() {
                near_bindgen::env::set_blockchain_interface(Box::new(near_blockchain::NearBlockchain {}));
                #[derive(serde :: Deserialize)]
                struct Input {
                    k: u64,
                }
                let Input { mut k, }: Input = serde_json::from_slice(
                    &near_bindgen::env::input().expect("Expected input since method has arguments.")
                )
                .expect("Failed to deserialize input from JSON.");
                let contract: Hello = near_bindgen::env::state_read().unwrap_or_default();
                contract.method(&mut k, );
            }
        );
        assert_eq!(expected.to_string(), actual.to_string());
    }

    #[test]
    fn callback_args() {
        let impl_type: Type = syn::parse_str("Hello").unwrap();
        let method: ImplItemMethod = parse_quote! {
            pub fn method(&self, #[callback] x: &mut u64, y: String, #[callback] z: Vec<u8>) { }
        };
        let method_info = ImplMethodInfo::new(method, impl_type, false).unwrap();
        let actual = method_wrapper(&method_info);
        let expected = quote!(
            #[cfg(target_arch = "wasm32")]
            #[no_mangle]
            pub extern "C" fn method() {
                near_bindgen::env::set_blockchain_interface(Box::new(near_blockchain::NearBlockchain {}));
                #[derive(serde :: Deserialize)]
                struct Input {
                    y: String,
                }
                let Input { y, }: Input = serde_json::from_slice(
                    &near_bindgen::env::input().expect("Expected input since method has arguments.")
                )
                .expect("Failed to deserialize input from JSON.");
                let data: Vec<u8> = match near_bindgen::env::promise_result(0usize) {
                    near_bindgen::PromiseResult::Successful(x) => x,
                    _ => panic!("Callback computation {} was not successful", 0usize)
                };
                let mut x: u64 =
                    serde_json::from_slice(&data).expect("Failed to deserialize callback using JSON");
                let data: Vec<u8> = match near_bindgen::env::promise_result(1usize) {
                    near_bindgen::PromiseResult::Successful(x) => x,
                    _ => panic!("Callback computation {} was not successful", 1usize)
                };
                let z: Vec<u8> =
                    serde_json::from_slice(&data).expect("Failed to deserialize callback using JSON");
                let contract: Hello = near_bindgen::env::state_read().unwrap_or_default();
                contract.method(&mut x, y, z, );
            }
        );
        assert_eq!(expected.to_string(), actual.to_string());
    }

    #[test]
    fn callback_args_only() {
        let impl_type: Type = syn::parse_str("Hello").unwrap();
        let method: ImplItemMethod = parse_quote! {
            pub fn method(&self, #[callback] x: &mut u64, #[callback] y: String) { }
        };
        let method_info = ImplMethodInfo::new(method, impl_type, false).unwrap();
        let actual = method_wrapper(&method_info);
        let expected = quote!(
            #[cfg(target_arch = "wasm32")]
            #[no_mangle]
            pub extern "C" fn method() {
                near_bindgen::env::set_blockchain_interface(Box::new(near_blockchain::NearBlockchain {}));
                let data: Vec<u8> = match near_bindgen::env::promise_result(0usize) {
                    near_bindgen::PromiseResult::Successful(x) => x,
                    _ => panic!("Callback computation {} was not successful", 0usize)
                };
                let mut x: u64 =
                    serde_json::from_slice(&data).expect("Failed to deserialize callback using JSON");
                let data: Vec<u8> = match near_bindgen::env::promise_result(1usize) {
                    near_bindgen::PromiseResult::Successful(x) => x,
                    _ => panic!("Callback computation {} was not successful", 1usize)
                };
                let y: String =
                    serde_json::from_slice(&data).expect("Failed to deserialize callback using JSON");
                let contract: Hello = near_bindgen::env::state_read().unwrap_or_default();
                contract.method(&mut x, y, );
            }
        );

        assert_eq!(expected.to_string(), actual.to_string());
    }

    #[test]
    fn callback_args_vec() {
        let impl_type: Type = syn::parse_str("Hello").unwrap();
        let method: ImplItemMethod = parse_quote! {
            pub fn method(&self, #[callback_vec] x: Vec<String>, y: String) { }
        };
        let method_info = ImplMethodInfo::new(method, impl_type, false).unwrap();
        let actual = method_wrapper(&method_info);
        let expected = quote!(
            #[cfg(target_arch = "wasm32")]
            #[no_mangle]
            pub extern "C" fn method() {
                near_bindgen::env::set_blockchain_interface(Box::new(near_blockchain::NearBlockchain {}));
                #[derive(serde :: Deserialize)]
                struct Input {
                    y: String,
                }
                let Input { y, }: Input = serde_json::from_slice(
                    &near_bindgen::env::input().expect("Expected input since method has arguments.")
                )
                .expect("Failed to deserialize input from JSON.");
                let x: Vec<String> = (0..near_bindgen::env::promise_results_count())
                    .map(|i| {
                        let data: Vec<u8> = match near_bindgen::env::promise_result(i) {
                            near_bindgen::PromiseResult::Successful(x) => x,
                            _ => panic!("Callback computation {} was not successful", i)
                        };
                        serde_json::from_slice(&data).expect("Failed to deserialize callback using JSON")
                    })
                    .collect();
                let contract: Hello = near_bindgen::env::state_read().unwrap_or_default();
                contract.method(x, y, );
            }
        );
        assert_eq!(expected.to_string(), actual.to_string());
    }

    #[test]
    fn simple_init() {
        let impl_type: Type = syn::parse_str("Hello").unwrap();
        let method: ImplItemMethod = parse_quote! {
            #[init]
            pub fn method(k: &mut u64) -> Self { }
        };
        let method_info = ImplMethodInfo::new(method, impl_type, false).unwrap();
        let actual = method_wrapper(&method_info);
        let expected = quote!(
            #[cfg(target_arch = "wasm32")]
            #[no_mangle]
            pub extern "C" fn method() {
                near_bindgen::env::set_blockchain_interface(Box::new(near_blockchain::NearBlockchain {}));
                #[derive(serde :: Deserialize)]
                struct Input {
                    k: u64,
                }
                let Input { mut k, }: Input = serde_json::from_slice(
                    &near_bindgen::env::input().expect("Expected input since method has arguments.")
                )
                .expect("Failed to deserialize input from JSON.");
                let contract = Hello::method(&mut k,);
                near_bindgen::env::state_write(&contract);
            }
        );
        assert_eq!(expected.to_string(), actual.to_string());
    }

    #[test]
    fn args_return_mut_borsh() {
        let impl_type: Type = syn::parse_str("Hello").unwrap();
        let method: ImplItemMethod = parse_quote! {
            #[result_serializer(borsh)]
            pub fn method(&mut self, #[serializer(borsh)] k: u64, #[serializer(borsh)]m: Bar) -> Option<u64> { }
        };
        let method_info = ImplMethodInfo::new(method, impl_type, false).unwrap();
        let actual = method_wrapper(&method_info);
        let expected = quote!(
            #[cfg(target_arch = "wasm32")]
            #[no_mangle]
            pub extern "C" fn method() {
                near_bindgen::env::set_blockchain_interface(Box::new(near_blockchain::NearBlockchain {}));
                #[derive(borsh :: BorshDeserialize)]
                struct Input {
                    k: u64,
                    m: Bar,
                }
                let Input { k, m, }: Input = borsh::Deserialize::try_from_slice(
                    &near_bindgen::env::input().expect("Expected input since method has arguments.")
                )
                .expect("Failed to deserialize input from Borsh.");
                let mut contract: Hello = near_bindgen::env::state_read().unwrap_or_default();
                let result = contract.method(k, m, );
                let result = borsh::BorshSerialize::try_to_vec(&contract, &result)
                    .expect("Failed to serialize the return value using Borsh.");
                near_bindgen::env::value_return(&result);
                near_bindgen::env::state_write(&contract);
            }
        );
        assert_eq!(expected.to_string(), actual.to_string());
    }

    #[test]
    fn callback_args_mixed_serialization() {
        let impl_type: Type = syn::parse_str("Hello").unwrap();
        let method: ImplItemMethod = parse_quote! {
            pub fn method(&self, #[callback] #[serializer(borsh)] x: &mut u64, #[serializer(borsh)] y: String, #[callback] #[serializer(json)] z: Vec<u8>) { }
        };
        let method_info = ImplMethodInfo::new(method, impl_type, false).unwrap();
        let actual = method_wrapper(&method_info);
        let expected = quote!(
            #[cfg(target_arch = "wasm32")]
            #[no_mangle]
            pub extern "C" fn method() {
                near_bindgen::env::set_blockchain_interface(Box::new(near_blockchain::NearBlockchain {}));
                #[derive(borsh :: BorshDeserialize)]
                struct Input {
                    y: String,
                }
                let Input { y, }: Input = borsh::Deserialize::try_from_slice(
                    &near_bindgen::env::input().expect("Expected input since method has arguments.")
                )
                .expect("Failed to deserialize input from Borsh.");
                let data: Vec<u8> = match near_bindgen::env::promise_result(0usize) {
                    near_bindgen::PromiseResult::Successful(x) => x,
                    _ => panic!("Callback computation {} was not successful", 0usize)
                };
                let mut x: u64 = borsh::Deserialize::try_from_slice(&data)
                    .expect("Failed to deserialize callback using JSON");
                let data: Vec<u8> = match near_bindgen::env::promise_result(1usize) {
                    near_bindgen::PromiseResult::Successful(x) => x,
                    _ => panic!("Callback computation {} was not successful", 1usize)
                };
                let z: Vec<u8> =
                    serde_json::from_slice(&data).expect("Failed to deserialize callback using JSON");
                let contract: Hello = near_bindgen::env::state_read().unwrap_or_default();
                contract.method(&mut x, y, z, );
            }
        );
        assert_eq!(expected.to_string(), actual.to_string());
    }
}
