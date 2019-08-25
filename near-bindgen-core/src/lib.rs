#![recursion_limit = "128"]
use proc_macro2::Span;
use quote::quote;
use syn::export::TokenStream2;
use syn::spanned::Spanned;
use syn::{Error, FnArg, ImplItem, ImplItemMethod, ItemImpl, Pat, ReturnType, Type, Visibility};

/// `env` is a keyword for near-bindgen.
const ENV_ARG: &str = "env";

const ENV_MUTABLE_REF_ERR: &str =
    "Special argument `env` should be passed as mutable reference, `&mut Environment`.";

/// Check that narrows down argument types and return type descriptive enough for deserialization and serialization.
fn check_arg_return_type(ty: &Type, span: Span) -> syn::Result<()> {
    match ty {
        Type::Slice(_)
        | Type::Array(_)
        | Type::Reference(_)
        | Type::Tuple(_)
        | Type::Path(_)
        | Type::Paren(_)
        | Type::Group(_) => Ok(()),

        Type::Ptr(_)
        | Type::BareFn(_)
        | Type::Never(_)
        | Type::TraitObject(_)
        | Type::ImplTrait(_)
        | Type::Infer(_)
        | Type::Macro(_)
        | Type::Verbatim(_) => Err(Error::new(
            span,
            "Not serializable/deserializable type of the smart contract argument/return type.",
        )),
    }
}

/// If method has arguments generates code to parse arguments.
/// # Returns:
/// * Code that parses arguments;
/// * List of arguments to be passed into the method of the object;
pub fn get_arg_parsing(method: &ImplItemMethod) -> syn::Result<(TokenStream2, TokenStream2)> {
    let mut result = TokenStream2::new();
    let mut result_args = TokenStream2::new();
    for arg in &method.sig.decl.inputs {
        match arg {
            // Allowed types of arguments.
            FnArg::SelfRef(_) | FnArg::SelfValue(_) => {}
            FnArg::Captured(arg) => {
                let arg_name = if let Pat::Ident(arg_name) = &arg.pat {
                    arg_name
                } else {
                    return Err(Error::new(arg.span(), "Unsupported argument name pattern."));
                };
                let arg_name_quoted = quote! { #arg_name }.to_string();

                if arg_name_quoted.as_str() == ENV_ARG {
                    if let Type::Reference(r) = &arg.ty {
                        if r.mutability.is_none() {
                            return Err(Error::new(arg.span(), ENV_MUTABLE_REF_ERR));
                        } else {
                            return Err(Error::new(arg.span(), ENV_MUTABLE_REF_ERR));
                        }
                    }
                    result_args.extend(quote! {
                        &mut #arg_name ,
                    });
                } else {
                    check_arg_return_type(&arg.ty, arg.span())?;

                    if let Type::Reference(r) = &arg.ty {
                        if r.mutability.is_some() {
                            result.extend(quote! {
                                let mut #arg = serde_json::from_value(args[#arg_name_quoted].clone()).unwrap();
                            });
                            result_args.extend(quote! {
                                &mut #arg_name ,
                            });
                        } else {
                            result.extend(quote! {
                                let #arg = serde_json::from_value(args[#arg_name_quoted].clone()).unwrap();
                            });
                            result_args.extend(quote! {
                                &#arg_name ,
                            });
                        };
                    } else {
                        result.extend(quote! {
                            let #arg = serde_json::from_value(args[#arg_name_quoted].clone()).unwrap();
                        });
                        result_args.extend(quote! {
                            #arg_name ,
                        });
                    }
                }
            }
            _ => return Err(Error::new(arg.span(), format!("Unsupported argument type"))),
        }
    }

    // If there are some args then add parsing header.
    if !result.is_empty() {
        result = quote! {
            let args: serde_json::Value = serde_json::from_slice(&env.input()).unwrap();
            #result
        };
    }
    Ok((result, result_args))
}

/// Checks whether the method should be considered to be a part of contract API.
pub fn is_contract_api(method: &ImplItemMethod, is_trait_impl: bool) -> bool {
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
        check_arg_return_type(return_type.as_ref(), span)?;
        match return_type.as_ref() {
            Type::Reference(_) => Ok(quote! {
                 let result = serde_json::to_vec(result).unwrap();
                 env.value_return(&result);
            }),
            _ => Ok(quote! {
                 let result = serde_json::to_vec(&result).unwrap();
                 env.value_return(&result);
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
) -> syn::Result<TokenStream2> {
    if !is_contract_api(method, is_trait_impl) {
        return Ok(TokenStream2::new());
    }
    if !method.sig.decl.generics.params.is_empty() {
        return Err(Error::new(
            method.sig.decl.generics.params.span(),
            "Methods exposed as contract API cannot use type parameters",
        ));
    }

    let (arg_parsing_code, arg_list) = get_arg_parsing(method)?;
    let return_code = get_return_serialization(&method.sig.decl.output)?;

    // Whether method uses self.
    let mut uses_self = false;
    // Code that reads and deserializes the state, if state is used.
    let mut state_de_code = TokenStream2::new();
    // Code that reads and serializes the state, if state was modified.
    let mut state_ser_code = TokenStream2::new();
    for arg in &method.sig.decl.inputs {
        match arg {
            FnArg::SelfRef(arg) => {
                uses_self = true;
                if arg.mutability.is_some() {
                    state_de_code = quote! {
                        let mut contract: #impl_type = env.state_read().unwrap_or_default();
                    };
                    state_ser_code = quote! {
                        env.state_write(&contract);
                    }
                } else {
                    state_de_code = quote! {
                        let contract: #impl_type = env.state_read().unwrap_or_default();
                    };
                }
            }
            FnArg::SelfValue(arg) => {
                uses_self = true;
                if arg.mutability.is_some() {
                    return Err(Error::new(
                        arg.span(),
                        "Cannot use `mut self` because method cannot consume `self` \
                         since we need it to record the change to the state. \
                         Either use reference or remove mutability.",
                    ));
                } else {
                    state_de_code = quote! {
                        let mut contract: #impl_type = env.state_read().unwrap_or_default();
                    };
                    state_ser_code = quote! {
                        env.state_write(&contract);
                    }
                }
            }
            _ => {}
        }
    }

    let env_creation = quote! {
        let mut env_val = near_bindgen::Environment::new(Box::new(near_blockchain::NearBlockchain{}));
        let env = &mut env_val;
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
        #env_creation
        #arg_parsing_code
        #state_de_code
        #method_invocation
        #return_code
        #state_ser_code
    };

    Ok(quote! {
        #[cfg(not(feature = "env_test"))]
        #[no_mangle]
        pub extern "C" fn #method_name() {
            #method_body
        }
    })
}

/// Processes `impl` section of the struct.
pub fn process_impl(item_impl: &ItemImpl) -> TokenStream2 {
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
            match process_method(m, impl_type, is_trait_impl) {
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
    use syn::{ImplItemMethod, Type};

    #[test]
    fn trait_implt() {
        let impl_type: Type = syn::parse_str("Hello").unwrap();
        let method: ImplItemMethod = syn::parse_str("fn method(&self) { }").unwrap();

        let actual = process_method(&method, &impl_type, true).unwrap();
        let expected = quote!(
            #[cfg(not(feature = "env_test"))]
            #[no_mangle]
            pub extern "C" fn method() {
                near_bindgen::ENV.set(Box::new(NearEnvironment{}));
                let mut contract: Hello = read_state().unwrap_or_default();
                let result = contract.method();
                write_state(&contract);
            }
        );
        assert_eq!(expected.to_string(), actual.to_string());
    }

    #[test]
    fn no_args_no_return_no_mut() {
        let impl_type: Type = syn::parse_str("Hello").unwrap();
        let method: ImplItemMethod = syn::parse_str("pub fn method(&self) { }").unwrap();

        let actual = process_method(&method, &impl_type, false).unwrap();
        let expected = quote!(
            #[cfg(not(feature = "env_test"))]
            #[no_mangle]
            pub extern "C" fn method() {
                near_bindgen::ENV.set(Box::new(NearEnvironment{}));
                let mut contract: Hello = read_state().unwrap_or_default();
                let result = contract.method();
                write_state(&contract);
            }
        );
        assert_eq!(expected.to_string(), actual.to_string());
    }

    #[test]
    fn no_args_no_return_mut() {
        let impl_type: Type = syn::parse_str("Hello").unwrap();
        let method: ImplItemMethod = syn::parse_str("pub fn method(&mut self) { }").unwrap();

        let actual = process_method(&method, &impl_type, false).unwrap();
        let expected = quote!(
            #[cfg(not(feature = "env_test"))]
            #[no_mangle]
            pub extern "C" fn method() {
                near_bindgen::ENV.set(Box::new(NearEnvironment{}));
                let mut contract: Hello = read_state().unwrap_or_default();
                let result = contract.method();
                write_state(&contract);
            }
        );
        assert_eq!(expected.to_string(), actual.to_string());
    }

    #[test]
    fn arg_no_return_no_mut() {
        let impl_type: Type = syn::parse_str("Hello").unwrap();
        let method: ImplItemMethod = syn::parse_str("pub fn method(&self, k: u64) { }").unwrap();

        let actual = process_method(&method, &impl_type, false).unwrap();
        let expected = quote!(
            #[cfg(not(feature = "env_test"))]
            #[no_mangle]
            pub extern "C" fn method() {
                near_bindgen::ENV.set(Box::new(NearEnvironment{}));
                let args: serde_json::Value = serde_json::from_slice(&near_bindgen::ENV.input()).unwrap();
                let k: u64 = serde_json::from_value(args["k"].clone()).unwrap();
                let mut contract: Hello = read_state().unwrap_or_default();
                let result = contract.method(k, );
                write_state(&contract);
            }
        );
        assert_eq!(expected.to_string(), actual.to_string());
    }

    #[test]
    fn args_no_return_mut() {
        let impl_type: Type = syn::parse_str("Hello").unwrap();
        let method: ImplItemMethod =
            syn::parse_str("pub fn method(&mut self, k: u64, m: Bar) { }").unwrap();

        let actual = process_method(&method, &impl_type, false).unwrap();
        let expected = quote!(
            #[cfg(not(feature = "env_test"))]
            #[no_mangle]
            pub extern "C" fn method() {
                near_bindgen::ENV.set(Box::new(NearEnvironment{}));
                let args: serde_json::Value = serde_json::from_slice(&near_bindgen::ENV.input()).unwrap();
                let k: u64 = serde_json::from_value(args["k"].clone()).unwrap();
                let m: Bar = serde_json::from_value(args["m"].clone()).unwrap();
                let mut contract: Hello = read_state().unwrap_or_default();
                let result = contract.method(k, m, );
                write_state(&contract);
            }
        );
        assert_eq!(expected.to_string(), actual.to_string());
    }

    #[test]
    fn args_return_mut() {
        let impl_type: Type = syn::parse_str("Hello").unwrap();
        let method: ImplItemMethod =
            syn::parse_str("pub fn method(&mut self, k: u64, m: Bar) -> Option<u64> { }").unwrap();

        let actual = process_method(&method, &impl_type, false).unwrap();
        let expected = quote!(
            #[cfg(not(feature = "env_test"))]
            #[no_mangle]
            pub extern "C" fn method() {
                near_bindgen::ENV.set(Box::new(NearEnvironment{}));
                let args: serde_json::Value = serde_json::from_slice(&near_bindgen::ENV.input()).unwrap();
                let k: u64 = serde_json::from_value(args["k"].clone()).unwrap();
                let m: Bar = serde_json::from_value(args["m"].clone()).unwrap();
                let mut contract: Hello = read_state().unwrap_or_default();
                let result = contract.method(k, m, );
                write_state(&contract);
                let result = serde_json::to_vec(&result).unwrap();
                unsafe {
                    near_bindgen::ENV.return_value(&result);
                }
            }
        );
        assert_eq!(expected.to_string(), actual.to_string());
    }

    #[test]
    fn args_return_ref() {
        let impl_type: Type = syn::parse_str("Hello").unwrap();
        let method: ImplItemMethod =
            syn::parse_str("pub fn method(&self) -> &Option<u64> { }").unwrap();

        let actual = process_method(&method, &impl_type, false).unwrap();
        let expected = quote!(
            #[cfg(not(feature = "env_test"))]
            #[no_mangle]
            pub extern "C" fn method() {
                near_bindgen::ENV.set(Box::new(NearEnvironment{}));
                let mut contract: Hello = read_state().unwrap_or_default();
                let result = contract.method();
                write_state(&contract);
                let result = serde_json::to_vec(result).unwrap();
                unsafe {
                    near_bindgen::ENV.return_value(&result);
                }
            }
        );
        assert_eq!(expected.to_string(), actual.to_string());
    }

    #[test]
    fn arg_ref() {
        let impl_type: Type = syn::parse_str("Hello").unwrap();
        let method: ImplItemMethod = syn::parse_str("pub fn method(&self, k: &u64) { }").unwrap();

        let actual = process_method(&method, &impl_type, false).unwrap();
        let expected = quote!(
            #[cfg(not(feature = "env_test"))]
            #[no_mangle]
            pub extern "C" fn method() {
                near_bindgen::ENV.set(Box::new(NearEnvironment{}));
                let args: serde_json::Value = serde_json::from_slice(&near_bindgen::ENV.input()).unwrap();
                let k: u64 = serde_json::from_value(args["k"].clone()).unwrap();
                let mut contract: Hello = read_state().unwrap_or_default();
                let result = contract.method(&k, );
                write_state(&contract);
            }
        );
        assert_eq!(expected.to_string(), actual.to_string());
    }

    #[test]
    fn arg_mut_ref() {
        let impl_type: Type = syn::parse_str("Hello").unwrap();
        let method: ImplItemMethod =
            syn::parse_str("pub fn method(&self, k: &mut u64) { }").unwrap();

        let actual = process_method(&method, &impl_type, false).unwrap();
        let expected = quote!(
            #[cfg(not(feature = "env_test"))]
            #[no_mangle]
            pub extern "C" fn method() {
                near_bindgen::ENV.set(Box::new(NearEnvironment{}));
                let args: serde_json::Value = serde_json::from_slice(&near_bindgen::ENV.input()).unwrap();
                let mut k: u64 = serde_json::from_value(args["k"].clone()).unwrap();
                let mut contract: Hello = read_state().unwrap_or_default();
                let result = contract.method(&mut k, );
                write_state(&contract);
            }
        );
        assert_eq!(expected.to_string(), actual.to_string());
    }
}
