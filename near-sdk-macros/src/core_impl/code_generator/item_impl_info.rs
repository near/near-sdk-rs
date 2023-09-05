use crate::core_impl::ext::generate_ext_function_wrappers;
use crate::ItemImplInfo;
use proc_macro2::TokenStream as TokenStream2;
use quote::ToTokens;
use syn::{spanned::Spanned, Ident};

impl ItemImplInfo {
    /// Generate the code that wraps
    pub fn wrapper_code(&self) -> TokenStream2 {
        let mut res = TokenStream2::new();
        for method in &self.methods {
            res.extend(method.method_wrapper());
        }
        res
    }

    pub fn generate_ext_wrapper_code(&self) -> TokenStream2 {
        match syn::parse::<Ident>(self.ty.to_token_stream().into()) {
            Ok(n) => generate_ext_function_wrappers(
                &n,
                self.methods.iter().map(|m| &m.attr_signature_info),
            ),
            Err(e) => syn::Error::new(self.ty.span(), e).to_compile_error(),
        }
    }
}
// Rustfmt removes comas.
#[rustfmt::skip]
#[cfg(test)]
mod tests {
    use syn::{Type, ImplItemMethod, parse_quote};
    use quote::quote;
    use crate::core_impl::info_extractor::ImplItemMethodInfo;


    #[test]
    fn trait_implt() {
        let impl_type: Type = syn::parse_str("Hello").unwrap();
        let mut method: ImplItemMethod = syn::parse_str("fn method(&self) { }").unwrap();
        let method_info = ImplItemMethodInfo::new(&mut method, true, impl_type).unwrap().unwrap();
        let actual = method_info.method_wrapper();
        let expected = quote!(
            #[cfg(target_arch = "wasm32")]
            #[no_mangle]
            pub extern "C" fn method() {
                ::near_sdk::env::setup_panic_hook();
                let contract: Hello = ::near_sdk::env::state_read().unwrap_or_default();
                contract.method();
            }
        );
        assert_eq!(expected.to_string(), actual.to_string());
    }

    #[test]
    fn no_args_no_return_no_mut() {
        let impl_type: Type = syn::parse_str("Hello").unwrap();
        let mut method: ImplItemMethod = syn::parse_str("pub fn method(&self) { }").unwrap();
        let method_info = ImplItemMethodInfo::new(&mut method, false, impl_type).unwrap().unwrap();
        let actual = method_info.method_wrapper();
        let expected = quote!(
            #[cfg(target_arch = "wasm32")]
            #[no_mangle]
            pub extern "C" fn method() {
                ::near_sdk::env::setup_panic_hook();
                let contract: Hello = ::near_sdk::env::state_read().unwrap_or_default();
                contract.method();
            }
        );
        assert_eq!(expected.to_string(), actual.to_string());
    }

    #[test]
    fn owned_no_args_no_return_no_mut() {
        let impl_type: Type = syn::parse_str("Hello").unwrap();
        let mut method: ImplItemMethod = syn::parse_str("pub fn method(self) { }").unwrap();
        let method_info = ImplItemMethodInfo::new(&mut method, false, impl_type).unwrap().unwrap();
        let actual = method_info.method_wrapper();
        let expected = quote!(
            #[cfg(target_arch = "wasm32")]
            #[no_mangle]
            pub extern "C" fn method() {
                ::near_sdk::env::setup_panic_hook();
                let contract: Hello = ::near_sdk::env::state_read().unwrap_or_default();
                contract.method();
            }
        );
        assert_eq!(expected.to_string(), actual.to_string());
    }


    #[test]
    fn mut_owned_no_args_no_return() {
        let impl_type: Type = syn::parse_str("Hello").unwrap();
        let mut method: ImplItemMethod = syn::parse_str("pub fn method(mut self) { }").unwrap();
        let method_info = ImplItemMethodInfo::new(&mut method, false, impl_type).unwrap().unwrap();
        let actual = method_info.method_wrapper();
        let expected = quote!(
            #[cfg(target_arch = "wasm32")]
            #[no_mangle]
            pub extern "C" fn method() {
                ::near_sdk::env::setup_panic_hook();
                let mut contract: Hello = ::near_sdk::env::state_read().unwrap_or_default();
                contract.method();
            }
        );
        assert_eq!(expected.to_string(), actual.to_string());
    }

    #[test]
    fn no_args_no_return_mut() {
        let impl_type: Type = syn::parse_str("Hello").unwrap();
        let mut method: ImplItemMethod = syn::parse_str("pub fn method(&mut self) { }").unwrap();
        let method_info = ImplItemMethodInfo::new(&mut method, false, impl_type).unwrap().unwrap();
        let actual = method_info.method_wrapper();
        let expected = quote!(
            #[cfg(target_arch = "wasm32")]
            #[no_mangle]
            pub extern "C" fn method() {
                ::near_sdk::env::setup_panic_hook();
                if ::near_sdk::env::attached_deposit() != 0 {
                    ::near_sdk::env::panic_str("Method method doesn't accept deposit");
                }
                let mut contract: Hello = ::near_sdk::env::state_read().unwrap_or_default();
                contract.method();
                ::near_sdk::env::state_write(&contract);
            }
        );
        assert_eq!(expected.to_string(), actual.to_string());
    }

    #[test]
    fn arg_no_return_no_mut() {
        let impl_type: Type = syn::parse_str("Hello").unwrap();
        let mut method: ImplItemMethod = syn::parse_str("pub fn method(&self, k: u64) { }").unwrap();
        let method_info = ImplItemMethodInfo::new(&mut method, false, impl_type).unwrap().unwrap();
        let actual = method_info.method_wrapper();
        let expected = quote!(
            #[cfg(target_arch = "wasm32")]
            #[no_mangle]
            pub extern "C" fn method() {
                ::near_sdk::env::setup_panic_hook();
                #[derive(::near_sdk :: serde :: Deserialize)]
                #[serde(crate = "::near_sdk::serde")]
                struct Input {
                    k: u64,
                }
                let Input { k, }: Input = ::near_sdk::serde_json::from_slice(
                    &::near_sdk::env::input().expect("Expected input since method has arguments.")
                )
                .expect("Failed to deserialize input from JSON.");
                let contract: Hello = ::near_sdk::env::state_read().unwrap_or_default();
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
        let method_info = ImplItemMethodInfo::new(&mut method, false, impl_type).unwrap().unwrap();
        let actual = method_info.method_wrapper();
        let expected = quote!(
                #[cfg(target_arch = "wasm32")]
                #[no_mangle]
                pub extern "C" fn method() {
                    ::near_sdk::env::setup_panic_hook();
                    if ::near_sdk::env::attached_deposit() != 0 {
                        ::near_sdk::env::panic_str("Method method doesn't accept deposit");
                    }
                    #[derive(::near_sdk :: serde :: Deserialize)]
                    #[serde(crate = "::near_sdk::serde")]
                    struct Input {
                        k: u64,
                        m: Bar,
                    }
                    let Input { k, m, }: Input = ::near_sdk::serde_json::from_slice(
                        &::near_sdk::env::input().expect("Expected input since method has arguments.")
                    )
                    .expect("Failed to deserialize input from JSON.");
                    let mut contract: Hello = ::near_sdk::env::state_read().unwrap_or_default();
                    contract.method(k, m, );
                    ::near_sdk::env::state_write(&contract);
                }
        );
        assert_eq!(expected.to_string(), actual.to_string());
    }

    #[test]
    fn args_return_mut() {
        let impl_type: Type = syn::parse_str("Hello").unwrap();
        let mut method: ImplItemMethod =
            syn::parse_str("pub fn method(&mut self, k: u64, m: Bar) -> Option<u64> { }").unwrap();
        let method_info = ImplItemMethodInfo::new(&mut method, false, impl_type).unwrap().unwrap();
        let actual = method_info.method_wrapper();
        let expected = quote!(
                #[cfg(target_arch = "wasm32")]
                #[no_mangle]
                pub extern "C" fn method() {
                    ::near_sdk::env::setup_panic_hook();
                    if ::near_sdk::env::attached_deposit() != 0 {
                        ::near_sdk::env::panic_str("Method method doesn't accept deposit");
                    }
                    #[derive(::near_sdk :: serde :: Deserialize)]
                    #[serde(crate = "::near_sdk::serde")]
                    struct Input {
                        k: u64,
                        m: Bar,
                    }
                    let Input { k, m, }: Input = ::near_sdk::serde_json::from_slice(
                        &::near_sdk::env::input().expect("Expected input since method has arguments.")
                    )
                    .expect("Failed to deserialize input from JSON.");
                    let mut contract: Hello = ::near_sdk::env::state_read().unwrap_or_default();
                    let result = contract.method(k, m, );
                    let result =
                        ::near_sdk::serde_json::to_vec(&result).expect("Failed to serialize the return value using JSON.");
                    ::near_sdk::env::value_return(&result);
                    ::near_sdk::env::state_write(&contract);
                }
        );
        assert_eq!(expected.to_string(), actual.to_string());
    }

    #[test]
    fn args_return_ref() {
        let impl_type: Type = syn::parse_str("Hello").unwrap();
        let mut method: ImplItemMethod =
            syn::parse_str("pub fn method(&self) -> &Option<u64> { }").unwrap();
        let method_info = ImplItemMethodInfo::new(&mut method, false, impl_type).unwrap().unwrap();
        let actual = method_info.method_wrapper();
        let expected = quote!(
            #[cfg(target_arch = "wasm32")]
            #[no_mangle]
            pub extern "C" fn method() {
                ::near_sdk::env::setup_panic_hook();
                let contract: Hello = ::near_sdk::env::state_read().unwrap_or_default();
                let result = contract.method();
                let result =
                    ::near_sdk::serde_json::to_vec(&result).expect("Failed to serialize the return value using JSON.");
                ::near_sdk::env::value_return(&result);
            }
        );
        assert_eq!(expected.to_string(), actual.to_string());
    }

    #[test]
    fn arg_ref() {
        let impl_type: Type = syn::parse_str("Hello").unwrap();
        let mut method: ImplItemMethod = syn::parse_str("pub fn method(&self, k: &u64) { }").unwrap();
        let method_info = ImplItemMethodInfo::new(&mut method, false, impl_type).unwrap().unwrap();
        let actual = method_info.method_wrapper();
        let expected = quote!(
                #[cfg(target_arch = "wasm32")]
                #[no_mangle]
                pub extern "C" fn method() {
                    ::near_sdk::env::setup_panic_hook();
                    #[derive(::near_sdk :: serde :: Deserialize)]
                    #[serde(crate = "::near_sdk::serde")]
                    struct Input {
                        k: u64,
                    }
                    let Input { k, }: Input = ::near_sdk::serde_json::from_slice(
                        &::near_sdk::env::input().expect("Expected input since method has arguments.")
                    )
                    .expect("Failed to deserialize input from JSON.");
                    let contract: Hello = ::near_sdk::env::state_read().unwrap_or_default();
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
        let method_info = ImplItemMethodInfo::new(&mut method, false, impl_type).unwrap().unwrap();
        let actual = method_info.method_wrapper();
        let expected = quote!(
            #[cfg(target_arch = "wasm32")]
            #[no_mangle]
            pub extern "C" fn method() {
                ::near_sdk::env::setup_panic_hook();
                #[derive(::near_sdk :: serde :: Deserialize)]
                #[serde(crate = "::near_sdk::serde")]
                struct Input {
                    k: u64,
                }
                let Input { mut k, }: Input = ::near_sdk::serde_json::from_slice(
                    &::near_sdk::env::input().expect("Expected input since method has arguments.")
                )
                .expect("Failed to deserialize input from JSON.");
                let contract: Hello = ::near_sdk::env::state_read().unwrap_or_default();
                contract.method(&mut k, );
            }
        );
        assert_eq!(expected.to_string(), actual.to_string());
    }

    #[test]
    fn callback_args() {
        let impl_type: Type = syn::parse_str("Hello").unwrap();
        let mut method: ImplItemMethod = parse_quote! {
            #[private] pub fn method(&self, #[callback_unwrap] x: &mut u64, y: ::std::string::String, #[callback_unwrap] z: ::std::vec::Vec<u8>) { }
        };
        let method_info = ImplItemMethodInfo::new(&mut method, false, impl_type).unwrap().unwrap();
        let actual = method_info.method_wrapper();
        let expected = quote!(
            #[cfg(target_arch = "wasm32")]
            #[no_mangle]
            pub extern "C" fn method() {
                ::near_sdk::env::setup_panic_hook();
                if ::near_sdk::env::current_account_id() != ::near_sdk::env::predecessor_account_id() {
                    ::near_sdk::env::panic_str("Method method is private");
                }
                #[derive(::near_sdk :: serde :: Deserialize)]
                #[serde(crate = "::near_sdk::serde")]
                struct Input {
                    y: ::std::string::String,
                }
                let Input { y, }: Input = ::near_sdk::serde_json::from_slice(
                    &::near_sdk::env::input().expect("Expected input since method has arguments.")
                )
                .expect("Failed to deserialize input from JSON.");
                let data: ::std::vec::Vec<u8> = match ::near_sdk::env::promise_result(0u64) {
                    ::near_sdk::PromiseResult::Successful(x) => x,
                    _ => ::near_sdk::env::panic_str("Callback computation 0 was not successful")
                };
                let mut x: u64 =
                    ::near_sdk::serde_json::from_slice(&data).expect("Failed to deserialize callback using JSON");
                let data: ::std::vec::Vec<u8> = match ::near_sdk::env::promise_result(1u64) {
                    ::near_sdk::PromiseResult::Successful(x) => x,
                    _ => ::near_sdk::env::panic_str("Callback computation 1 was not successful")
                };
                let z: ::std::vec::Vec<u8> =
                    ::near_sdk::serde_json::from_slice(&data).expect("Failed to deserialize callback using JSON");
                let contract: Hello = ::near_sdk::env::state_read().unwrap_or_default();
                contract.method(&mut x, y, z, );
            }
        );
        assert_eq!(expected.to_string(), actual.to_string());
    }

    #[test]
    fn callback_args_only() {
        let impl_type: Type = syn::parse_str("Hello").unwrap();
        let mut method: ImplItemMethod = parse_quote! {
            #[private] pub fn method(&self, #[callback_unwrap] x: &mut u64, #[callback_unwrap] y: ::std::string::String) { }
        };
        let method_info = ImplItemMethodInfo::new(&mut method, false, impl_type).unwrap().unwrap();
        let actual = method_info.method_wrapper();
        let expected = quote!(
            #[cfg(target_arch = "wasm32")]
            #[no_mangle]
            pub extern "C" fn method() {
                ::near_sdk::env::setup_panic_hook();
                if ::near_sdk::env::current_account_id() != ::near_sdk::env::predecessor_account_id() {
                    ::near_sdk::env::panic_str("Method method is private");
                }
                let data: ::std::vec::Vec<u8> = match ::near_sdk::env::promise_result(0u64) {
                    ::near_sdk::PromiseResult::Successful(x) => x,
                    _ => ::near_sdk::env::panic_str("Callback computation 0 was not successful")
                };
                let mut x: u64 =
                    ::near_sdk::serde_json::from_slice(&data).expect("Failed to deserialize callback using JSON");
                let data: ::std::vec::Vec<u8> = match ::near_sdk::env::promise_result(1u64) {
                    ::near_sdk::PromiseResult::Successful(x) => x,
                    _ => ::near_sdk::env::panic_str("Callback computation 1 was not successful")
                };
                let y: ::std::string::String =
                    ::near_sdk::serde_json::from_slice(&data).expect("Failed to deserialize callback using JSON");
                let contract: Hello = ::near_sdk::env::state_read().unwrap_or_default();
                contract.method(&mut x, y, );
            }
        );

        assert_eq!(expected.to_string(), actual.to_string());
    }

    #[test]
    fn callback_args_results() {
        let impl_type: Type = syn::parse_str("Hello").unwrap();
        let mut method: ImplItemMethod = parse_quote! {
            #[private] pub fn method(&self, #[callback_result] x: &mut Result<u64, PromiseError>, #[callback_result] y: Result<::std::string::String, PromiseError>) { }
        };
        let method_info = ImplItemMethodInfo::new(&mut method, false, impl_type).unwrap().unwrap();
        let actual = method_info.method_wrapper();
        let expected = quote!(
            #[cfg(target_arch = "wasm32")]
            #[no_mangle]
            pub extern "C" fn method() {
                ::near_sdk::env::setup_panic_hook();
                if ::near_sdk::env::current_account_id() != ::near_sdk::env::predecessor_account_id() {
                    ::near_sdk::env::panic_str("Method method is private");
                }
                let mut x: Result<u64, PromiseError> = match ::near_sdk::env::promise_result(0u64) {
                    ::near_sdk::PromiseResult::Successful(data) => ::std::result::Result::Ok(::near_sdk::serde_json::from_slice(&data).expect("Failed to deserialize callback using JSON")),
                    ::near_sdk::PromiseResult::Failed => ::std::result::Result::Err(::near_sdk::PromiseError::Failed),
                };
                let y: Result<::std::string::String, PromiseError> = match ::near_sdk::env::promise_result(1u64) {
                    ::near_sdk::PromiseResult::Successful(data) => ::std::result::Result::Ok(::near_sdk::serde_json::from_slice(&data).expect("Failed to deserialize callback using JSON")),
                    ::near_sdk::PromiseResult::Failed => ::std::result::Result::Err(::near_sdk::PromiseError::Failed),
                };
                let contract: Hello = ::near_sdk::env::state_read().unwrap_or_default();
                contract.method(&mut x, y, );
            }
        );

        assert_eq!(expected.to_string(), actual.to_string());
    }

    #[test]
    fn callback_args_vec() {
        let impl_type: Type = syn::parse_str("Hello").unwrap();
        let mut method: ImplItemMethod = parse_quote! {
            #[private] pub fn method(&self, #[callback_vec] x: Vec<String>, y: String) { }
        };
        let method_info = ImplItemMethodInfo::new(&mut method, false, impl_type).unwrap().unwrap();
        let actual = method_info.method_wrapper();
        let expected = quote!(
            #[cfg(target_arch = "wasm32")]
            #[no_mangle]
            pub extern "C" fn method() {
                ::near_sdk::env::setup_panic_hook();
                if ::near_sdk::env::current_account_id() != ::near_sdk::env::predecessor_account_id() {
                    ::near_sdk::env::panic_str("Method method is private");
                }
                #[derive(::near_sdk :: serde :: Deserialize)]
                #[serde(crate = "::near_sdk::serde")]
                struct Input {
                    y: String,
                }
                let Input { y, }: Input = ::near_sdk::serde_json::from_slice(
                    &::near_sdk::env::input().expect("Expected input since method has arguments.")
                )
                .expect("Failed to deserialize input from JSON.");
                let x: Vec<String> = ::std::iter::Iterator::collect(::std::iter::Iterator::map(0..::near_sdk::env::promise_results_count(), |i| {
                        let data: ::std::vec::Vec<u8> = match ::near_sdk::env::promise_result(i) {
                            ::near_sdk::PromiseResult::Successful(x) => x,
                            _ => ::near_sdk::env::panic_str(&::std::format!("Callback computation {} was not successful", i)),
                        };
                        ::near_sdk::serde_json::from_slice(&data).expect("Failed to deserialize callback using JSON")
                    }));
                let contract: Hello = ::near_sdk::env::state_read().unwrap_or_default();
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
        let method_info = ImplItemMethodInfo::new(&mut method, false, impl_type).unwrap().unwrap();
        let actual = method_info.method_wrapper();
        let expected = quote!(
            #[cfg(target_arch = "wasm32")]
            #[no_mangle]
            pub extern "C" fn method() {
                ::near_sdk::env::setup_panic_hook();
                if ::near_sdk::env::attached_deposit() != 0 {
                    ::near_sdk::env::panic_str("Method method doesn't accept deposit");
                }
                #[derive(::near_sdk :: serde :: Deserialize)]
                #[serde(crate = "::near_sdk::serde")]
                struct Input {
                    k: u64,
                }
                let Input { mut k, }: Input = ::near_sdk::serde_json::from_slice(
                    &::near_sdk::env::input().expect("Expected input since method has arguments.")
                )
                .expect("Failed to deserialize input from JSON.");
                if ::near_sdk::env::state_exists() {
                    ::near_sdk::env::panic_str("The contract has already been initialized");
                }
                let contract = Hello::method(&mut k,);
                ::near_sdk::env::state_write(&contract);
            }
        );
        assert_eq!(expected.to_string(), actual.to_string());
    }

    #[test]
    fn init_no_return() {
        let impl_type: Type = syn::parse_str("Hello").unwrap();
        let mut method: ImplItemMethod = parse_quote! {
            #[init]
            pub fn method(k: &mut u64) { }
        };
        let actual = ImplItemMethodInfo::new(&mut method, false, impl_type).map(|_| ()).unwrap_err();
        let expected = "Init function must return the contract state.";
        assert_eq!(expected, actual.to_string());
    }

    #[test]
    fn init_ignore_state() {
        let impl_type: Type = syn::parse_str("Hello").unwrap();
        let mut method: ImplItemMethod = parse_quote! {
            #[init(ignore_state)]
            pub fn method(k: &mut u64) -> Self { }
        };
        let method_info = ImplItemMethodInfo::new(&mut method, false, impl_type).unwrap().unwrap();
        let actual = method_info.method_wrapper();
        let expected = quote!(
            #[cfg(target_arch = "wasm32")]
            #[no_mangle]
            pub extern "C" fn method() {
                ::near_sdk::env::setup_panic_hook();
                if ::near_sdk::env::attached_deposit() != 0 {
                    ::near_sdk::env::panic_str("Method method doesn't accept deposit");
                }
                #[derive(::near_sdk :: serde :: Deserialize)]
                #[serde(crate = "::near_sdk::serde")]
                struct Input {
                    k: u64,
                }
                let Input { mut k, }: Input = ::near_sdk::serde_json::from_slice(
                    &::near_sdk::env::input().expect("Expected input since method has arguments.")
                )
                .expect("Failed to deserialize input from JSON.");
                let contract = Hello::method(&mut k,);
                ::near_sdk::env::state_write(&contract);
            }
        );
        assert_eq!(expected.to_string(), actual.to_string());
    }

    #[test]
    fn init_payable() {
        let impl_type: Type = syn::parse_str("Hello").unwrap();
        let mut method: ImplItemMethod = parse_quote! {
            #[init]
            #[payable]
            pub fn method(k: &mut u64) -> Self { }
        };
        let method_info = ImplItemMethodInfo::new(&mut method, false, impl_type).unwrap().unwrap();
        let actual = method_info.method_wrapper();
        let expected = quote!(
            #[cfg(target_arch = "wasm32")]
            #[no_mangle]
            pub extern "C" fn method() {
                ::near_sdk::env::setup_panic_hook();
                #[derive(::near_sdk :: serde :: Deserialize)]
                #[serde(crate = "::near_sdk::serde")]
                struct Input {
                    k: u64,
                }
                let Input { mut k, }: Input = ::near_sdk::serde_json::from_slice(
                    &::near_sdk::env::input().expect("Expected input since method has arguments.")
                )
                .expect("Failed to deserialize input from JSON.");
                if ::near_sdk::env::state_exists() {
                    ::near_sdk::env::panic_str("The contract has already been initialized");
                }
                let contract = Hello::method(&mut k,);
                ::near_sdk::env::state_write(&contract);
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
        let method_info = ImplItemMethodInfo::new(&mut method, false, impl_type).unwrap().unwrap();
        let actual = method_info.method_wrapper();
        let expected = quote!(
            #[cfg(target_arch = "wasm32")]
            #[no_mangle]
            pub extern "C" fn method() {
                ::near_sdk::env::setup_panic_hook();
                if ::near_sdk::env::attached_deposit() != 0 {
                    ::near_sdk::env::panic_str("Method method doesn't accept deposit");
                }
                #[derive(::near_sdk :: borsh :: BorshDeserialize)]
                #[borsh(crate = "::near_sdk::borsh")]
                struct Input {
                    k: u64,
                    m: Bar,
                }
                let Input { k, m, }: Input = ::near_sdk::borsh::BorshDeserialize::try_from_slice(
                    &::near_sdk::env::input().expect("Expected input since method has arguments.")
                )
                .expect("Failed to deserialize input from Borsh.");
                let mut contract: Hello = ::near_sdk::env::state_read().unwrap_or_default();
                let result = contract.method(k, m, );
                let result = ::near_sdk::borsh::to_vec(&result)
                    .expect("Failed to serialize the return value using Borsh.");
                ::near_sdk::env::value_return(&result);
                ::near_sdk::env::state_write(&contract);
            }
        );
        assert_eq!(expected.to_string(), actual.to_string());
    }

    #[test]
    fn callback_args_mixed_serialization() {
        let impl_type: Type = syn::parse_str("Hello").unwrap();
        let mut method: ImplItemMethod = parse_quote! {
            #[private] pub fn method(&self, #[callback_unwrap] #[serializer(borsh)] x: &mut u64, #[serializer(borsh)] y: ::std::string::String, #[callback_unwrap] #[serializer(json)] z: ::std::vec::Vec<u8>) { }
        };
        let method_info = ImplItemMethodInfo::new(&mut method, false, impl_type).unwrap().unwrap();
        let actual = method_info.method_wrapper();
        let expected = quote!(
            #[cfg(target_arch = "wasm32")]
            #[no_mangle]
            pub extern "C" fn method() {
                ::near_sdk::env::setup_panic_hook();
                if ::near_sdk::env::current_account_id() != ::near_sdk::env::predecessor_account_id() {
                    ::near_sdk::env::panic_str("Method method is private");
                }
                #[derive(::near_sdk :: borsh :: BorshDeserialize)]
                #[borsh(crate = "::near_sdk::borsh")]
                struct Input {
                    y: ::std::string::String,
                }
                let Input { y, }: Input = ::near_sdk::borsh::BorshDeserialize::try_from_slice(
                    &::near_sdk::env::input().expect("Expected input since method has arguments.")
                )
                .expect("Failed to deserialize input from Borsh.");
                let data: ::std::vec::Vec<u8> = match ::near_sdk::env::promise_result(0u64) {
                    ::near_sdk::PromiseResult::Successful(x) => x,
                    _ => ::near_sdk::env::panic_str("Callback computation 0 was not successful")
                };
                let mut x: u64 = ::near_sdk::borsh::BorshDeserialize::try_from_slice(&data)
                    .expect("Failed to deserialize callback using Borsh");
                let data: ::std::vec::Vec<u8> = match ::near_sdk::env::promise_result(1u64) {
                    ::near_sdk::PromiseResult::Successful(x) => x,
                    _ => ::near_sdk::env::panic_str("Callback computation 1 was not successful")
                };
                let z: ::std::vec::Vec<u8> =
                    ::near_sdk::serde_json::from_slice(&data).expect("Failed to deserialize callback using JSON");
                let contract: Hello = ::near_sdk::env::state_read().unwrap_or_default();
                contract.method(&mut x, y, z, );
            }
        );
        assert_eq!(expected.to_string(), actual.to_string());
    }

    #[test]
    fn no_args_no_return_mut_payable() {
        let impl_type: Type = syn::parse_str("Hello").unwrap();
        let mut method: ImplItemMethod = syn::parse_str("#[payable] pub fn method(&mut self) { }").unwrap();
        let method_info = ImplItemMethodInfo::new(&mut method, false, impl_type).unwrap().unwrap();
        let actual = method_info.method_wrapper();
        let expected = quote!(
            #[cfg(target_arch = "wasm32")]
            #[no_mangle]
            pub extern "C" fn method() {
                ::near_sdk::env::setup_panic_hook();
                let mut contract: Hello = ::near_sdk::env::state_read().unwrap_or_default();
                contract.method();
                ::near_sdk::env::state_write(&contract);
            }
        );
        assert_eq!(expected.to_string(), actual.to_string());
    }

    #[test]
    fn private_method() {
        let impl_type: Type = syn::parse_str("Hello").unwrap();
        let mut method: ImplItemMethod = syn::parse_str("#[private] pub fn private_method(&mut self) { }").unwrap();
        let method_info = ImplItemMethodInfo::new(&mut method, false, impl_type).unwrap().unwrap();
        let actual = method_info.method_wrapper();
        let expected = quote!(
            #[cfg(target_arch = "wasm32")]
            #[no_mangle]
            pub extern "C" fn private_method() {
                ::near_sdk::env::setup_panic_hook();
                if ::near_sdk::env::current_account_id() != ::near_sdk::env::predecessor_account_id() {
                    ::near_sdk::env::panic_str("Method private_method is private");
                }
                if ::near_sdk::env::attached_deposit() != 0 {
                    ::near_sdk::env::panic_str("Method private_method doesn't accept deposit");
                }
                let mut contract: Hello = ::near_sdk::env::state_read().unwrap_or_default();
                contract.private_method();
                ::near_sdk::env::state_write(&contract);
            }
        );
        assert_eq!(expected.to_string(), actual.to_string());
    }

    #[test]
    fn handle_result_json() {
        let impl_type: Type = syn::parse_str("Hello").unwrap();
        let mut method: ImplItemMethod = parse_quote! {
            #[handle_result]
            pub fn method(&self) -> Result::<u64, &'static str> { }
        };
        let method_info = ImplItemMethodInfo::new(&mut method, false, impl_type).unwrap().unwrap();
        let actual = method_info.method_wrapper();
        let expected = quote!(
            #[cfg(target_arch = "wasm32")]
            #[no_mangle]
            pub extern "C" fn method() {
                ::near_sdk::env::setup_panic_hook();
                let contract: Hello = ::near_sdk::env::state_read().unwrap_or_default();
                let result = contract.method();
                match result {
                    ::std::result::Result::Ok(result) => {
                        let result =
                            ::near_sdk::serde_json::to_vec(&result).expect("Failed to serialize the return value using JSON.");
                        ::near_sdk::env::value_return(&result);
                    }
                    ::std::result::Result::Err(err) => ::near_sdk::FunctionError::panic(&err)
                }
            }
        );
        assert_eq!(expected.to_string(), actual.to_string());
    }
    
    #[test]
    fn handle_result_mut() {
        let impl_type: Type = syn::parse_str("Hello").unwrap();
        let mut method: ImplItemMethod = parse_quote! {
            #[handle_result]
            pub fn method(&mut self) -> Result<u64, &'static str> { }
        };
        let method_info = ImplItemMethodInfo::new(&mut method, false, impl_type).unwrap().unwrap();
        let actual = method_info.method_wrapper();
        let expected = quote!(
            #[cfg(target_arch = "wasm32")]
            #[no_mangle]
            pub extern "C" fn method() {
                ::near_sdk::env::setup_panic_hook();
                if ::near_sdk::env::attached_deposit() != 0 {
                    ::near_sdk::env::panic_str("Method method doesn't accept deposit");
                }
                let mut contract: Hello = ::near_sdk::env::state_read().unwrap_or_default();
                let result = contract.method();
                match result {
                    ::std::result::Result::Ok(result) => {
                        let result =
                            ::near_sdk::serde_json::to_vec(&result).expect("Failed to serialize the return value using JSON.");
                        ::near_sdk::env::value_return(&result);
                        ::near_sdk::env::state_write(&contract);
                    }
                    ::std::result::Result::Err(err) => ::near_sdk::FunctionError::panic(&err)
                }
            }
        );
        assert_eq!(expected.to_string(), actual.to_string());
    }

    #[test]
    fn handle_result_borsh() {
        let impl_type: Type = syn::parse_str("Hello").unwrap();
        let mut method: ImplItemMethod = parse_quote! {
            #[handle_result]
            #[result_serializer(borsh)]
            pub fn method(&self) -> Result<u64, &'static str> { }
        };
        let method_info = ImplItemMethodInfo::new(&mut method, false, impl_type).unwrap().unwrap();
        let actual = method_info.method_wrapper();
        let expected = quote!(
            #[cfg(target_arch = "wasm32")]
            #[no_mangle]
            pub extern "C" fn method() {
                ::near_sdk::env::setup_panic_hook();
                let contract: Hello = ::near_sdk::env::state_read().unwrap_or_default();
                let result = contract.method();
                match result {
                    ::std::result::Result::Ok(result) => {
                        let result =
                            ::near_sdk::borsh::to_vec(&result).expect("Failed to serialize the return value using Borsh.");
                        ::near_sdk::env::value_return(&result);
                    }
                    ::std::result::Result::Err(err) => ::near_sdk::FunctionError::panic(&err)
                }
            }
        );
        assert_eq!(expected.to_string(), actual.to_string());
    }

    #[test]
    fn handle_result_init() {
        let impl_type: Type = syn::parse_str("Hello").unwrap();
        let mut method: ImplItemMethod = parse_quote! {
            #[init]
            #[handle_result]
            pub fn new() -> Result<Self, &'static str> { }
        };
        let method_info = ImplItemMethodInfo::new(&mut method, false, impl_type).unwrap().unwrap();
        let actual = method_info.method_wrapper();
        let expected = quote!(
            #[cfg(target_arch = "wasm32")]
            #[no_mangle]
            pub extern "C" fn new() {
                ::near_sdk::env::setup_panic_hook();
                if ::near_sdk::env::attached_deposit() != 0 {
                    ::near_sdk::env::panic_str("Method new doesn't accept deposit");
                }
                if ::near_sdk::env::state_exists() {
                    ::near_sdk::env::panic_str("The contract has already been initialized");
                }
                let contract = Hello::new();
                match contract {
                    ::std::result::Result::Ok(contract) => {
                        ::near_sdk::env::state_write(&contract);
                    }
                    ::std::result::Result::Err(err) => ::near_sdk::FunctionError::panic(&err)
                }
            }
        );
        assert_eq!(expected.to_string(), actual.to_string());
    }

    #[test]
    fn handle_result_init_ignore_state() {
        let impl_type: Type = syn::parse_str("Hello").unwrap();
        let mut method: ImplItemMethod = parse_quote! {
            #[init(ignore_state)]
            #[handle_result]
            pub fn new() -> Result<Self, &'static str> { }
        };
        let method_info = ImplItemMethodInfo::new(&mut method, false, impl_type).unwrap().unwrap();
        let actual = method_info.method_wrapper();
        let expected = quote!(
            #[cfg(target_arch = "wasm32")]
            #[no_mangle]
            pub extern "C" fn new() {
                ::near_sdk::env::setup_panic_hook();
                if ::near_sdk::env::attached_deposit() != 0 {
                    ::near_sdk::env::panic_str("Method new doesn't accept deposit");
                }
                let contract = Hello::new();
                match contract {
                    ::std::result::Result::Ok(contract) => {
                        ::near_sdk::env::state_write(&contract);
                    }
                    ::std::result::Result::Err(err) => ::near_sdk::FunctionError::panic(&err)
                }
            }
        );
        assert_eq!(expected.to_string(), actual.to_string());
    }

    #[test]
    fn handle_no_self() {
        let impl_type: Type = syn::parse_str("Hello").unwrap();
        let mut method: ImplItemMethod = syn::parse_str("pub fn method() { }").unwrap();
        let method_info = ImplItemMethodInfo::new(&mut method, false, impl_type).unwrap().unwrap();
        let actual = method_info.method_wrapper();
        let expected = quote!(
            #[cfg(target_arch = "wasm32")]
            #[no_mangle]
            pub extern "C" fn method() {
                ::near_sdk::env::setup_panic_hook();
                Hello::method();
            }
        );
        assert_eq!(expected.to_string(), actual.to_string());
    }
}
