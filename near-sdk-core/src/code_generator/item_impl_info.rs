use crate::ItemImplInfo;
use syn::export::TokenStream2;

impl ItemImplInfo {
    /// Generate the code that wraps
    pub fn wrapper_code(&self) -> TokenStream2 {
        let mut res = TokenStream2::new();
        for method in &self.methods {
            if method.is_public || self.is_trait_impl {
                res.extend(method.method_wrapper());
            }
        }
        res
    }
}
// Rustfmt removes comas.
#[rustfmt::skip]
#[cfg(test)]
mod tests {
    use syn::{Type, ImplItemMethod, parse_quote};
    use quote::quote;
    use crate::info_extractor::ImplItemMethodInfo;


    #[test]
    fn trait_implt() {
        let impl_type: Type = syn::parse_str("Hello").unwrap();
        let mut method: ImplItemMethod = syn::parse_str("fn method(&self) { }").unwrap();
        let method_info = ImplItemMethodInfo::new(&mut method, impl_type).unwrap();
        let actual = method_info.method_wrapper();
        let expected = quote!(
            #[cfg(target_arch = "wasm32")]
            #[no_mangle]
            pub extern "C" fn method() {
                near_sdk::env::setup_panic_hook();
                near_sdk::env::set_blockchain_interface(Box::new(near_blockchain::NearBlockchain {}));
                let contract: Hello = near_sdk::env::state_read().unwrap_or_default();
                contract.method();
            }
        );
        assert_eq!(expected.to_string(), actual.to_string());
    }

    #[test]
    fn no_args_no_return_no_mut() {
        let impl_type: Type = syn::parse_str("Hello").unwrap();
        let mut method: ImplItemMethod = syn::parse_str("pub fn method(&self) { }").unwrap();
        let method_info = ImplItemMethodInfo::new(&mut method, impl_type).unwrap();
        let actual = method_info.method_wrapper();
        let expected = quote!(
            #[cfg(target_arch = "wasm32")]
            #[no_mangle]
            pub extern "C" fn method() {
                near_sdk::env::setup_panic_hook();
                near_sdk::env::set_blockchain_interface(Box::new(near_blockchain::NearBlockchain {}));
                let contract: Hello = near_sdk::env::state_read().unwrap_or_default();
                contract.method();
            }
        );
        assert_eq!(expected.to_string(), actual.to_string());
    }

    #[test]
    fn no_args_no_return_mut() {
        let impl_type: Type = syn::parse_str("Hello").unwrap();
        let mut method: ImplItemMethod = syn::parse_str("pub fn method(&mut self) { }").unwrap();
        let method_info = ImplItemMethodInfo::new(&mut method, impl_type).unwrap();
        let actual = method_info.method_wrapper();
        let expected = quote!(
            #[cfg(target_arch = "wasm32")]
            #[no_mangle]
            pub extern "C" fn method() {
                near_sdk::env::setup_panic_hook();
                near_sdk::env::set_blockchain_interface(Box::new(near_blockchain::NearBlockchain {}));
                let mut contract: Hello = near_sdk::env::state_read().unwrap_or_default();
                contract.method();
                near_sdk::env::state_write(&contract);
            }
        );
        assert_eq!(expected.to_string(), actual.to_string());
    }

    #[test]
    fn arg_no_return_no_mut() {
        let impl_type: Type = syn::parse_str("Hello").unwrap();
        let mut method: ImplItemMethod = syn::parse_str("pub fn method(&self, k: u64) { }").unwrap();
        let method_info = ImplItemMethodInfo::new(&mut method, impl_type).unwrap();
        let actual = method_info.method_wrapper();
        let expected = quote!(
            #[cfg(target_arch = "wasm32")]
            #[no_mangle]
            pub extern "C" fn method() {
                near_sdk::env::setup_panic_hook();
                near_sdk::env::set_blockchain_interface(Box::new(near_blockchain::NearBlockchain {}));
                #[derive(serde :: Deserialize, serde :: Serialize)]
                struct Input {
                    k: u64,
                }
                let Input { k, }: Input = serde_json::from_slice(
                    &near_sdk::env::input().expect("Expected input since method has arguments.")
                )
                .expect("Failed to deserialize input from JSON.");
                let contract: Hello = near_sdk::env::state_read().unwrap_or_default();
                contract.method(k, );
            }
        );
        assert_eq!(expected.to_string(), actual.to_string());
    }



    #[test]
    fn args_no_return_mut() {
        let impl_type: Type = syn::parse_str("Hello").unwrap();
        let mut method: ImplItemMethod =
            syn::parse_str("pub fn method(&mut self, k: u64, m: Bar) { }").unwrap();
        let method_info = ImplItemMethodInfo::new(&mut method, impl_type).unwrap();
        let actual = method_info.method_wrapper();
        let expected = quote!(
                #[cfg(target_arch = "wasm32")]
                #[no_mangle]
                pub extern "C" fn method() {
                    near_sdk::env::setup_panic_hook();
                    near_sdk::env::set_blockchain_interface(Box::new(near_blockchain::NearBlockchain {}));
                    #[derive(serde :: Deserialize, serde :: Serialize)]
                    struct Input {
                        k: u64,
                        m: Bar,
                    }
                    let Input { k, m, }: Input = serde_json::from_slice(
                        &near_sdk::env::input().expect("Expected input since method has arguments.")
                    )
                    .expect("Failed to deserialize input from JSON.");
                    let mut contract: Hello = near_sdk::env::state_read().unwrap_or_default();
                    contract.method(k, m, );
                    near_sdk::env::state_write(&contract);
                }
        );
        assert_eq!(expected.to_string(), actual.to_string());
    }

    #[test]
    fn args_return_mut() {
        let impl_type: Type = syn::parse_str("Hello").unwrap();
        let mut method: ImplItemMethod =
            syn::parse_str("pub fn method(&mut self, k: u64, m: Bar) -> Option<u64> { }").unwrap();
        let method_info = ImplItemMethodInfo::new(&mut method, impl_type).unwrap();
        let actual = method_info.method_wrapper();
        let expected = quote!(
                #[cfg(target_arch = "wasm32")]
                #[no_mangle]
                pub extern "C" fn method() {
                    near_sdk::env::setup_panic_hook();
                    near_sdk::env::set_blockchain_interface(Box::new(near_blockchain::NearBlockchain {}));
                    #[derive(serde :: Deserialize, serde :: Serialize)]
                    struct Input {
                        k: u64,
                        m: Bar,
                    }
                    let Input { k, m, }: Input = serde_json::from_slice(
                        &near_sdk::env::input().expect("Expected input since method has arguments.")
                    )
                    .expect("Failed to deserialize input from JSON.");
                    let mut contract: Hello = near_sdk::env::state_read().unwrap_or_default();
                    let result = contract.method(k, m, );
                    let result =
                        serde_json::to_vec(&result).expect("Failed to serialize the return value using JSON.");
                    near_sdk::env::value_return(&result);
                    near_sdk::env::state_write(&contract);
                }
        );
        assert_eq!(expected.to_string(), actual.to_string());
    }

    #[test]
    fn args_return_ref() {
        let impl_type: Type = syn::parse_str("Hello").unwrap();
        let mut method: ImplItemMethod =
            syn::parse_str("pub fn method(&self) -> &Option<u64> { }").unwrap();
        let method_info = ImplItemMethodInfo::new(&mut method, impl_type).unwrap();
        let actual = method_info.method_wrapper();
        let expected = quote!(
            #[cfg(target_arch = "wasm32")]
            #[no_mangle]
            pub extern "C" fn method() {
                near_sdk::env::setup_panic_hook();
                near_sdk::env::set_blockchain_interface(Box::new(near_blockchain::NearBlockchain {}));
                let contract: Hello = near_sdk::env::state_read().unwrap_or_default();
                let result = contract.method();
                let result =
                    serde_json::to_vec(&result).expect("Failed to serialize the return value using JSON.");
                near_sdk::env::value_return(&result);
            }
        );
        assert_eq!(expected.to_string(), actual.to_string());
    }

    #[test]
    fn arg_ref() {
        let impl_type: Type = syn::parse_str("Hello").unwrap();
        let mut method: ImplItemMethod = syn::parse_str("pub fn method(&self, k: &u64) { }").unwrap();
        let method_info = ImplItemMethodInfo::new(&mut method, impl_type).unwrap();
        let actual = method_info.method_wrapper();
        let expected = quote!(
                #[cfg(target_arch = "wasm32")]
                #[no_mangle]
                pub extern "C" fn method() {
                    near_sdk::env::setup_panic_hook();
                    near_sdk::env::set_blockchain_interface(Box::new(near_blockchain::NearBlockchain {}));
                    #[derive(serde :: Deserialize, serde :: Serialize)]
                    struct Input {
                        k: u64,
                    }
                    let Input { k, }: Input = serde_json::from_slice(
                        &near_sdk::env::input().expect("Expected input since method has arguments.")
                    )
                    .expect("Failed to deserialize input from JSON.");
                    let contract: Hello = near_sdk::env::state_read().unwrap_or_default();
                    contract.method(&k, );
                }
        );
        assert_eq!(expected.to_string(), actual.to_string());
    }

    #[test]
    fn arg_mut_ref() {
        let impl_type: Type = syn::parse_str("Hello").unwrap();
        let mut method: ImplItemMethod =
            syn::parse_str("pub fn method(&self, k: &mut u64) { }").unwrap();
        let method_info = ImplItemMethodInfo::new(&mut method, impl_type).unwrap();
        let actual = method_info.method_wrapper();
        let expected = quote!(
            #[cfg(target_arch = "wasm32")]
            #[no_mangle]
            pub extern "C" fn method() {
                near_sdk::env::setup_panic_hook();
                near_sdk::env::set_blockchain_interface(Box::new(near_blockchain::NearBlockchain {}));
                #[derive(serde :: Deserialize, serde :: Serialize)]
                struct Input {
                    k: u64,
                }
                let Input { mut k, }: Input = serde_json::from_slice(
                    &near_sdk::env::input().expect("Expected input since method has arguments.")
                )
                .expect("Failed to deserialize input from JSON.");
                let contract: Hello = near_sdk::env::state_read().unwrap_or_default();
                contract.method(&mut k, );
            }
        );
        assert_eq!(expected.to_string(), actual.to_string());
    }

    #[test]
    fn callback_args() {
        let impl_type: Type = syn::parse_str("Hello").unwrap();
        let mut method: ImplItemMethod = parse_quote! {
            pub fn method(&self, #[callback] x: &mut u64, y: String, #[callback] z: Vec<u8>) { }
        };
        let method_info = ImplItemMethodInfo::new(&mut method, impl_type).unwrap();
        let actual = method_info.method_wrapper();
        let expected = quote!(
            #[cfg(target_arch = "wasm32")]
            #[no_mangle]
            pub extern "C" fn method() {
                near_sdk::env::setup_panic_hook();
                near_sdk::env::set_blockchain_interface(Box::new(near_blockchain::NearBlockchain {}));
                #[derive(serde :: Deserialize, serde :: Serialize)]
                struct Input {
                    y: String,
                }
                let Input { y, }: Input = serde_json::from_slice(
                    &near_sdk::env::input().expect("Expected input since method has arguments.")
                )
                .expect("Failed to deserialize input from JSON.");
                let data: Vec<u8> = match near_sdk::env::promise_result(0u64) {
                    near_sdk::PromiseResult::Successful(x) => x,
                    _ => panic!("Callback computation {} was not successful", 0u64)
                };
                let mut x: u64 =
                    serde_json::from_slice(&data).expect("Failed to deserialize callback using JSON");
                let data: Vec<u8> = match near_sdk::env::promise_result(1u64) {
                    near_sdk::PromiseResult::Successful(x) => x,
                    _ => panic!("Callback computation {} was not successful", 1u64)
                };
                let z: Vec<u8> =
                    serde_json::from_slice(&data).expect("Failed to deserialize callback using JSON");
                let contract: Hello = near_sdk::env::state_read().unwrap_or_default();
                contract.method(&mut x, y, z, );
            }
        );
        assert_eq!(expected.to_string(), actual.to_string());
    }

    #[test]
    fn callback_args_only() {
        let impl_type: Type = syn::parse_str("Hello").unwrap();
        let mut method: ImplItemMethod = parse_quote! {
            pub fn method(&self, #[callback] x: &mut u64, #[callback] y: String) { }
        };
        let method_info = ImplItemMethodInfo::new(&mut method, impl_type).unwrap();
        let actual = method_info.method_wrapper();
        let expected = quote!(
            #[cfg(target_arch = "wasm32")]
            #[no_mangle]
            pub extern "C" fn method() {
                near_sdk::env::setup_panic_hook();
                near_sdk::env::set_blockchain_interface(Box::new(near_blockchain::NearBlockchain {}));
                let data: Vec<u8> = match near_sdk::env::promise_result(0u64) {
                    near_sdk::PromiseResult::Successful(x) => x,
                    _ => panic!("Callback computation {} was not successful", 0u64)
                };
                let mut x: u64 =
                    serde_json::from_slice(&data).expect("Failed to deserialize callback using JSON");
                let data: Vec<u8> = match near_sdk::env::promise_result(1u64) {
                    near_sdk::PromiseResult::Successful(x) => x,
                    _ => panic!("Callback computation {} was not successful", 1u64)
                };
                let y: String =
                    serde_json::from_slice(&data).expect("Failed to deserialize callback using JSON");
                let contract: Hello = near_sdk::env::state_read().unwrap_or_default();
                contract.method(&mut x, y, );
            }
        );

        assert_eq!(expected.to_string(), actual.to_string());
    }

    #[test]
    fn callback_args_vec() {
        let impl_type: Type = syn::parse_str("Hello").unwrap();
        let mut method: ImplItemMethod = parse_quote! {
            pub fn method(&self, #[callback_vec] x: Vec<String>, y: String) { }
        };
        let method_info = ImplItemMethodInfo::new(&mut method, impl_type).unwrap();
        let actual = method_info.method_wrapper();
        let expected = quote!(
            #[cfg(target_arch = "wasm32")]
            #[no_mangle]
            pub extern "C" fn method() {
                near_sdk::env::setup_panic_hook();
                near_sdk::env::set_blockchain_interface(Box::new(near_blockchain::NearBlockchain {}));
                #[derive(serde :: Deserialize, serde :: Serialize)]
                struct Input {
                    y: String,
                }
                let Input { y, }: Input = serde_json::from_slice(
                    &near_sdk::env::input().expect("Expected input since method has arguments.")
                )
                .expect("Failed to deserialize input from JSON.");
                let x: Vec<String> = (0..near_sdk::env::promise_results_count())
                    .map(|i| {
                        let data: Vec<u8> = match near_sdk::env::promise_result(i) {
                            near_sdk::PromiseResult::Successful(x) => x,
                            _ => panic!("Callback computation {} was not successful", i)
                        };
                        serde_json::from_slice(&data).expect("Failed to deserialize callback using JSON")
                    })
                    .collect();
                let contract: Hello = near_sdk::env::state_read().unwrap_or_default();
                contract.method(x, y, );
            }
        );
        assert_eq!(expected.to_string(), actual.to_string());
    }

    #[test]
    fn simple_init() {
        let impl_type: Type = syn::parse_str("Hello").unwrap();
        let mut method: ImplItemMethod = parse_quote! {
            #[init]
            pub fn method(k: &mut u64) -> Self { }
        };
        let method_info = ImplItemMethodInfo::new(&mut method, impl_type).unwrap();
        let actual = method_info.method_wrapper();
        let expected = quote!(
            #[cfg(target_arch = "wasm32")]
            #[no_mangle]
            pub extern "C" fn method() {
                near_sdk::env::setup_panic_hook();
                near_sdk::env::set_blockchain_interface(Box::new(near_blockchain::NearBlockchain {}));
                #[derive(serde :: Deserialize, serde :: Serialize)]
                struct Input {
                    k: u64,
                }
                let Input { mut k, }: Input = serde_json::from_slice(
                    &near_sdk::env::input().expect("Expected input since method has arguments.")
                )
                .expect("Failed to deserialize input from JSON.");
                let contract = Hello::method(&mut k,);
                near_sdk::env::state_write(&contract);
            }
        );
        assert_eq!(expected.to_string(), actual.to_string());
    }

    #[test]
    fn args_return_mut_borsh() {
        let impl_type: Type = syn::parse_str("Hello").unwrap();
        let mut method: ImplItemMethod = parse_quote! {
            #[result_serializer(borsh)]
            pub fn method(&mut self, #[serializer(borsh)] k: u64, #[serializer(borsh)]m: Bar) -> Option<u64> { }
        };
        let method_info = ImplItemMethodInfo::new(&mut method, impl_type).unwrap();
        let actual = method_info.method_wrapper();
        let expected = quote!(
            #[cfg(target_arch = "wasm32")]
            #[no_mangle]
            pub extern "C" fn method() {
                near_sdk::env::setup_panic_hook();
                near_sdk::env::set_blockchain_interface(Box::new(near_blockchain::NearBlockchain {}));
                #[derive(borsh :: BorshDeserialize, borsh :: BorshSerialize)]
                struct Input {
                    k: u64,
                    m: Bar,
                }
                let Input { k, m, }: Input = borsh::BorshDeserialize::try_from_slice(
                    &near_sdk::env::input().expect("Expected input since method has arguments.")
                )
                .expect("Failed to deserialize input from Borsh.");
                let mut contract: Hello = near_sdk::env::state_read().unwrap_or_default();
                let result = contract.method(k, m, );
                let result = borsh::BorshSerialize::try_to_vec(&result)
                    .expect("Failed to serialize the return value using Borsh.");
                near_sdk::env::value_return(&result);
                near_sdk::env::state_write(&contract);
            }
        );
        assert_eq!(expected.to_string(), actual.to_string());
    }

    #[test]
    fn callback_args_mixed_serialization() {
        let impl_type: Type = syn::parse_str("Hello").unwrap();
        let mut method: ImplItemMethod = parse_quote! {
            pub fn method(&self, #[callback] #[serializer(borsh)] x: &mut u64, #[serializer(borsh)] y: String, #[callback] #[serializer(json)] z: Vec<u8>) { }
        };
        let method_info = ImplItemMethodInfo::new(&mut method, impl_type).unwrap();
        let actual = method_info.method_wrapper();
        let expected = quote!(
            #[cfg(target_arch = "wasm32")]
            #[no_mangle]
            pub extern "C" fn method() {
                near_sdk::env::setup_panic_hook();
                near_sdk::env::set_blockchain_interface(Box::new(near_blockchain::NearBlockchain {}));
                #[derive(borsh :: BorshDeserialize, borsh :: BorshSerialize)]
                struct Input {
                    y: String,
                }
                let Input { y, }: Input = borsh::BorshDeserialize::try_from_slice(
                    &near_sdk::env::input().expect("Expected input since method has arguments.")
                )
                .expect("Failed to deserialize input from Borsh.");
                let data: Vec<u8> = match near_sdk::env::promise_result(0u64) {
                    near_sdk::PromiseResult::Successful(x) => x,
                    _ => panic!("Callback computation {} was not successful", 0u64)
                };
                let mut x: u64 = borsh::BorshDeserialize::try_from_slice(&data)
                    .expect("Failed to deserialize callback using Borsh");
                let data: Vec<u8> = match near_sdk::env::promise_result(1u64) {
                    near_sdk::PromiseResult::Successful(x) => x,
                    _ => panic!("Callback computation {} was not successful", 1u64)
                };
                let z: Vec<u8> =
                    serde_json::from_slice(&data).expect("Failed to deserialize callback using JSON");
                let contract: Hello = near_sdk::env::state_read().unwrap_or_default();
                contract.method(&mut x, y, z, );
            }
        );
        assert_eq!(expected.to_string(), actual.to_string());
    }
}
