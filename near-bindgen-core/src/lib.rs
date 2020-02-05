#![recursion_limit = "128"]
use crate::initializer_attribute::{process_init_method, InitAttr};
use quote::quote;
use syn::export::{ToTokens, TokenStream2};
use syn::spanned::Spanned;
use syn::{
    Error, FnArg, GenericParam, ImplItem, ImplItemMethod, ItemImpl, Receiver, ReturnType, Type,
    Visibility,
};

mod arg_parsing;
mod callback_args;
mod callback_args_vec;
pub mod initializer_attribute;

/// Checks whether the method should be considered to be a part of contract API.
pub fn publicly_accessible(method: &ImplItemMethod, is_trait_impl: bool) -> bool {
    if let Visibility::Public(_) = method.vis {
        true
    } else {
        is_trait_impl
    }
}

/// Get code to serialize the return value.
pub fn get_return_serialization(return_type: &ReturnType) -> syn::Result<TokenStream2> {
    let span = return_type.span();
    if let ReturnType::Type(_, return_type) = return_type {
        arg_parsing::check_arg_return_type(return_type.as_ref(), span)?;
        match return_type.as_ref() {
            Type::Reference(_) => Ok(quote! {
                 let result = serde_json::to_vec(result).unwrap();
                 near_bindgen::env::value_return(&result);
            }),
            _ => Ok(quote! {
                 let result = serde_json::to_vec(&result).unwrap();
                 near_bindgen::env::value_return(&result);
            }),
        }
    } else {
        Ok(TokenStream2::new())
    }
}

/// Attempts processing `impl` method. If method is `pub` and has `&self`, `&mut self` or `self`
/// then it is considered to be a part of contract API, otherwise no tokens are is returned. If
/// method is a valid contract API then we examine its arguments and fail if they use complex
/// pattern matching.
pub fn process_method(
    method: &ImplItemMethod,
    impl_type: &Type,
    is_trait_impl: bool,
    has_init_method: bool,
) -> syn::Result<TokenStream2> {
    let attrs = method.attrs.iter().fold(TokenStream2::new(), |mut acc, attr| {
        let attr_str = attr.path.to_token_stream().to_string();
        if attr_str != "callback_args_vec" && attr_str != "callback_args" {
            attr.to_tokens(&mut acc);
        }
        acc
    });

    // If init method is declared we do not use `Default::default` to unwrap the state, even if
    // `Default` trait is implemented.
    let state_unwrapper = if has_init_method {
        quote! {unwrap()}
    } else {
        quote! {unwrap_or_default()}
    };
    if !publicly_accessible(method, is_trait_impl) {
        return Ok(TokenStream2::new());
    }
    if method.sig.generics.params.iter().any(|p| match p {
        GenericParam::Type(_) => true,
        _ => false,
    }) {
        return Err(Error::new(
            method.sig.generics.params.span(),
            "Methods exposed as contract API cannot use type parameters",
        ));
    }

    let (arg_parsing_code, arg_list) = arg_parsing::get_arg_parsing(method)?;
    let return_code = get_return_serialization(&method.sig.output)?;

    // Whether method uses self.
    let mut uses_self = false;
    // Code that reads and deserializes the state, if state is used.
    let mut state_de_code = TokenStream2::new();
    // Code that reads and serializes the state, if state was modified.
    let mut state_ser_code = TokenStream2::new();
    for arg in &method.sig.inputs {
        match arg {
            FnArg::Receiver(Receiver { reference: Some(_), mutability, .. }) => {
                uses_self = true;
                if mutability.is_some() {
                    state_de_code = quote! {
                        let mut contract: #impl_type = near_bindgen::env::state_read().#state_unwrapper;
                    };
                    state_ser_code = quote! {
                        near_bindgen::env::state_write(&contract);
                    }
                } else {
                    state_de_code = quote! {
                        let contract: #impl_type = near_bindgen::env::state_read().#state_unwrapper;
                    };
                }
            }
            FnArg::Receiver(Receiver { reference: None, mutability, .. }) => {
                uses_self = true;
                if mutability.is_some() {
                    return Err(Error::new(
                        arg.span(),
                        "Cannot use `mut self` because method cannot consume `self` \
                         since we need it to record the change to the state. \
                         Either use reference or remove mutability.",
                    ));
                } else {
                    state_de_code = quote! {
                        let mut contract: #impl_type = near_bindgen::env::state_read().#state_unwrapper;
                    };
                    state_ser_code = quote! {
                        near_bindgen::env::state_write(&contract);
                    }
                }
            }
            _ => {}
        }
    }

    let panic_hook = quote! {
         near_bindgen::env::setup_panic_hook();
    };

    let env_creation = quote! {
        near_bindgen::env::set_blockchain_interface(Box::new(near_blockchain::NearBlockchain {}));
    };

    let method_name = &method.sig.ident;
    // Depending on whether method is static or not we call it differently.
    let method_invocation = if uses_self {
        if return_code.is_empty() {
            quote! {
                contract.#method_name(#arg_list);
            }
        } else {
            quote! {
                let result = contract.#method_name(#arg_list);
            }
        }
    } else {
        if return_code.is_empty() {
            quote! {
                #impl_type::#method_name(#arg_list);
            }
        } else {
            quote! {
                let result = #impl_type::#method_name(#arg_list);
            }
        }
    };

    let method_body = quote! {
        #panic_hook
        #env_creation
        #arg_parsing_code
        #state_de_code
        #method_invocation
        #return_code
        #state_ser_code
    };

    Ok(quote! {
        #attrs
        #[cfg(target_arch = "wasm32")]
        #[no_mangle]
        pub extern "C" fn #method_name() {
            #method_body
        }
    })
}

/// Processes `impl` section of the struct.
/// # Args:
/// `item_impl` -- tokens representing `impl .. {}` body;
/// `attr` -- tokens representing attributes of the macro.
pub fn process_impl(item_impl: &ItemImpl, attr: TokenStream2) -> TokenStream2 {
    let init_attr = if attr.is_empty() {
        None
    } else {
        match syn::parse2::<InitAttr>(attr) {
            Ok(x) => Some(x),
            Err(err) => {
                return err.to_compile_error();
            }
        }
    };
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
    for subitem in &item_impl.items {
        if let ImplItem::Method(m) = subitem {
            let res = match &init_attr {
                Some(init_attr) if m.sig.ident.to_string() == init_attr.ident.to_string() => {
                    process_init_method(m, impl_type, is_trait_impl)
                }
                _ => process_method(m, impl_type, is_trait_impl, init_attr.is_some()),
            };
            match res {
                Ok(wrapped_method) => output.extend(wrapped_method),
                Err(err) => {
                    output.extend(err.to_compile_error());
                    break;
                }
            }
        }
    }
    output
}

// Rustfmt removes comas.
#[rustfmt::skip]
#[cfg(test)]
mod tests {
    use crate::process_method;
    use quote::quote;
    use syn::{ImplItemMethod, Type, parse_quote};

    #[test]
    fn trait_implt() {
        let impl_type: Type = syn::parse_str("Hello").unwrap();
        let method: ImplItemMethod = syn::parse_str("fn method(&self) { }").unwrap();

        let actual = process_method(&method, &impl_type, true, false).unwrap();
        let expected = quote!(
            #[cfg(target_arch = "wasm32")]
            #[no_mangle]
            pub extern "C" fn method() {
                near_bindgen::env::setup_panic_hook();
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

        let actual = process_method(&method, &impl_type, false, false).unwrap();
        let expected = quote!(
            #[cfg(target_arch = "wasm32")]
            #[no_mangle]
            pub extern "C" fn method() {
                near_bindgen::env::setup_panic_hook();
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

        let actual = process_method(&method, &impl_type, false, false).unwrap();
        let expected = quote!(
            #[cfg(target_arch = "wasm32")]
            #[no_mangle]
            pub extern "C" fn method() {
                near_bindgen::env::setup_panic_hook();
                near_bindgen::env::set_blockchain_interface(Box::new(near_blockchain::NearBlockchain {}));
                let mut contract: Hello = near_bindgen::env::state_read().unwrap_or_default();
                contract.method();
                near_bindgen::env::state_write(&contract);
            }
        );
        assert_eq!(expected.to_string(), actual.to_string());
    }

    #[test]
    fn no_args_no_return_mut_init() {
        let impl_type: Type = syn::parse_str("Hello").unwrap();
        let method: ImplItemMethod = syn::parse_str("pub fn method(&mut self) { }").unwrap();

        let actual = process_method(&method, &impl_type, false, true).unwrap();
        let expected = quote!(
            #[cfg(target_arch = "wasm32")]
            #[no_mangle]
            pub extern "C" fn method() {
                near_bindgen::env::setup_panic_hook();
                near_bindgen::env::set_blockchain_interface(Box::new(near_blockchain::NearBlockchain {}));
                let mut contract: Hello = near_bindgen::env::state_read().unwrap();
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

        let actual = process_method(&method, &impl_type, false, false).unwrap();
        let expected = quote!(
            #[cfg(target_arch = "wasm32")]
            #[no_mangle]
            pub extern "C" fn method() {
                near_bindgen::env::setup_panic_hook();
                near_bindgen::env::set_blockchain_interface(Box::new(near_blockchain::NearBlockchain {}));
                let args: serde_json::Value = serde_json::from_slice(&near_bindgen::env::input().unwrap()).unwrap();
                let k: u64 = serde_json::from_value(args["k"].clone()).unwrap();
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

        let actual = process_method(&method, &impl_type, false, false).unwrap();
        let expected = quote!(
            #[cfg(target_arch = "wasm32")]
            #[no_mangle]
            pub extern "C" fn method() {
                near_bindgen::env::setup_panic_hook();
                near_bindgen::env::set_blockchain_interface(Box::new(near_blockchain::NearBlockchain {}));
                let args: serde_json::Value = serde_json::from_slice(&near_bindgen::env::input().unwrap()).unwrap();
                let k: u64 = serde_json::from_value(args["k"].clone()).unwrap();
                let m: Bar = serde_json::from_value(args["m"].clone()).unwrap();
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

        let actual = process_method(&method, &impl_type, false, false).unwrap();
        let expected = quote!(
            #[cfg(target_arch = "wasm32")]
            #[no_mangle]
            pub extern "C" fn method() {
                near_bindgen::env::setup_panic_hook();
                near_bindgen::env::set_blockchain_interface(Box::new(near_blockchain::NearBlockchain {}));
                let args: serde_json::Value = serde_json::from_slice(&near_bindgen::env::input().unwrap()).unwrap();
                let k: u64 = serde_json::from_value(args["k"].clone()).unwrap();
                let m: Bar = serde_json::from_value(args["m"].clone()).unwrap();
                let mut contract: Hello = near_bindgen::env::state_read().unwrap_or_default();
                let result = contract.method(k, m, );
                let result = serde_json::to_vec(&result).unwrap();
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

        let actual = process_method(&method, &impl_type, false, false).unwrap();
        let expected = quote!(
            #[cfg(target_arch = "wasm32")]
            #[no_mangle]
            pub extern "C" fn method() {
                near_bindgen::env::setup_panic_hook();
                near_bindgen::env::set_blockchain_interface(Box::new(near_blockchain::NearBlockchain {}));
                let contract: Hello = near_bindgen::env::state_read().unwrap_or_default();
                let result = contract.method();
                let result = serde_json::to_vec(result).unwrap();
                near_bindgen::env::value_return(&result);
            }
        );
        assert_eq!(expected.to_string(), actual.to_string());
    }

    #[test]
    fn arg_ref() {
        let impl_type: Type = syn::parse_str("Hello").unwrap();
        let method: ImplItemMethod = syn::parse_str("pub fn method(&self, k: &u64) { }").unwrap();

        let actual = process_method(&method, &impl_type, false, false).unwrap();
        let expected = quote!(
            #[cfg(target_arch = "wasm32")]
            #[no_mangle]
            pub extern "C" fn method() {
                near_bindgen::env::setup_panic_hook();
                near_bindgen::env::set_blockchain_interface(Box::new(near_blockchain::NearBlockchain {}));
                let args: serde_json::Value = serde_json::from_slice(&near_bindgen::env::input().unwrap()).unwrap();
                let k: u64 = serde_json::from_value(args["k"].clone()).unwrap();
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

        let actual = process_method(&method, &impl_type, false, false).unwrap();
        let expected = quote!(
            #[cfg(target_arch = "wasm32")]
            #[no_mangle]
            pub extern "C" fn method() {
                near_bindgen::env::setup_panic_hook();
                near_bindgen::env::set_blockchain_interface(Box::new(near_blockchain::NearBlockchain {}));
                let args: serde_json::Value = serde_json::from_slice(&near_bindgen::env::input().unwrap()).unwrap();
                let mut k: u64 = serde_json::from_value(args["k"].clone()).unwrap();
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
            #[callback_args(x, z)]
            pub fn method(&self, x: &mut u64, y: String, z: Vec<u8>) { }
        };

        let actual = process_method(&method, &impl_type, false, false).unwrap();
        let expected = quote!(
            #[cfg(target_arch = "wasm32")]
            #[no_mangle]
            pub extern "C" fn method() {
                near_bindgen::env::setup_panic_hook();
                near_bindgen::env::set_blockchain_interface(Box::new(near_blockchain::NearBlockchain {}));
                let args: serde_json::Value = serde_json::from_slice(&near_bindgen::env::input().unwrap()).unwrap();
                assert_eq!(near_bindgen::env::promise_results_count(), 2u64);
                let data: Vec<u8> = match near_bindgen::env::promise_result(0u64) {
                    near_bindgen::PromiseResult::Successful(x) => x,
                    _ => panic!("Callback computation {} was not successful", 0u64)
                };
                let mut x: u64 = serde_json::from_slice(&data).unwrap();
                let y: String = serde_json::from_value(args["y"].clone()).unwrap();
                let data: Vec<u8> = match near_bindgen::env::promise_result(1u64) {
                    near_bindgen:: PromiseResult::Successful(x) => x,
                    _ => panic!("Callback computation {} was not successful", 1u64)
                };
                let z: Vec<u8> = serde_json::from_slice(&data).unwrap();
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
            #[callback_args(x, y)]
            pub fn method(&self, x: &mut u64, y: String) { }
        };

        // When there is no input args we should not even attempt reading input and parsing json
        // from it.
        let actual = process_method(&method, &impl_type, false, false).unwrap();
        let expected = quote!(
            #[cfg(target_arch = "wasm32")]
            #[no_mangle]
            pub extern "C" fn method() {
                near_bindgen::env::setup_panic_hook();
                near_bindgen::env::set_blockchain_interface(Box::new(near_blockchain::NearBlockchain {}));
                assert_eq!(near_bindgen::env::promise_results_count(), 2u64);
                let data: Vec<u8> = match near_bindgen::env::promise_result(0u64) {
                    near_bindgen::PromiseResult::Successful(x) => x,
                    _ => panic!("Callback computation {} was not successful", 0u64)
                };
                let mut x: u64 = serde_json::from_slice(&data).unwrap();
                let data: Vec<u8> = match near_bindgen::env::promise_result(1u64) {
                    near_bindgen:: PromiseResult::Successful(x) => x,
                    _ => panic!("Callback computation {} was not successful", 1u64)
                };
                let y: String = serde_json::from_slice(&data).unwrap();
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
            #[callback_args_vec(x)]
            pub fn method(&self, x: Vec<String>, y: String) { }
        };

        let actual = process_method(&method, &impl_type, false, false).unwrap();
        let expected = quote!(
            #[cfg(target_arch = "wasm32")]
            #[no_mangle]
            pub extern "C" fn method() {
                near_bindgen::env::setup_panic_hook();
                near_bindgen::env::set_blockchain_interface(Box::new(near_blockchain::NearBlockchain {}));
                let args: serde_json::Value = serde_json::from_slice(&near_bindgen::env::input().unwrap()).unwrap();
                let x: Vec<String> = (0..near_bindgen::env::promise_results_count())
                            .map(|i| {
                                let data: Vec<u8> = match near_bindgen::env::promise_result(i) {
                                    near_bindgen::PromiseResult::Successful(x) => x,
                                    _ => panic!("Callback computation {} was not successful", i)
                                };
                                serde_json::from_slice(&data).unwrap()
                            }).collect(); 
                let y: String = serde_json::from_value(args["y"].clone()).unwrap();
                let contract: Hello = near_bindgen::env::state_read().unwrap_or_default();
                contract.method(x, y, );
            }
        );
        assert_eq!(expected.to_string(), actual.to_string());
    }
}
