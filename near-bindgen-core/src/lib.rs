#![recursion_limit = "128"]
use crate::arg_deser::create_input_struct;
use crate::callback_args::CallbackArgs;
use crate::callback_args_vec::CallbackArgsVec;
use crate::initializer_attribute::{process_init_method, InitAttr};
use crate::serializer_attr::SerializerAttr;
use quote::quote;
use syn::export::{ToTokens, TokenStream2};
use syn::spanned::Spanned;
use syn::token::Token;
use syn::{
    Attribute, Error, FnArg, GenericParam, ImplItem, ImplItemMethod, ItemImpl, PatType, Receiver,
    ReturnType, Type, Visibility,
};

/// Type of serialization we use.
enum SerializerType {
    JSON,
    Borsh,
}

mod arg_deser;
mod callback_args;
mod callback_args_vec;
mod serializer_attr;

struct BindgenMethod {
    /// List of attributes that are not used by near-bindgen.
    non_bindgen_attrs: Vec<Attribute>,
    /// The list of arguments used to parse the input from the callback.
    callback_args: Vec<CallbackArgs>,
    /// An optional argument used to parse the entire input from the callback.
    callback_args_vec: Option<CallbackArgsVec>,
    /// Attribute defining serializer.
    serializer: Option<SerializerAttr>,
    /// Whether method can be used as initializer.
    is_init: bool,
    /// Regular arguments that are not callback and not self.
    regular_args: Vec<PatType>,
    /// Whether method has `pub` modifier or a part of trait implementation.
    is_public: bool,
    /// What this function returns.
    returns: ReturnType,
    /// The receiver, like `mut self`, `self`, `&mut self`, `&self`, or `None`.
    receiver: Option<Receiver>,
}

/// Process the method and extract information important for near-bindgen.
fn _extract_bindgen_info_from_method(
    method: &ImplItemMethod,
    impl_type: &Type,
    is_trait_impl: bool,
) -> syn::Result<BindgenMethod> {
    let mut callback_args = vec![];
    let mut callback_args_vec = None;
    let mut serializer = None;
    let mut is_init = false;
    let mut non_bindgen_attrs = vec![];
    let mut regular_args = vec![];
    for attr in &method.attrs {
        let attr_str = attr.path.to_token_stream().to_string().as_str();
        match attr_str {
            "init" => {
                is_init = true;
            }
            "callback_args" => callback_args.extend(syn::parse2(attr.tokens.clone())?),
            "callback_args_vec" => callback_args_vec = Some(syn::parse2(attr.tokens.clone())?),
            "serializer" => serializer = Some(syn::parse2(attr.tokens.clone())?),
            _ => non_bindgen_attrs.push((*attr).clone()),
        }
    }

    let is_public = match method.vis {
        Visibility::Public(_) => true,
        _ => is_trait_impl,
    };
    let returns = method.sig.output.clone();
    let mut receiver = None;
    for fn_arg in &method.sig.inputs {
        match fn_arg {
            FnArg::Receiver(r) => receiver = Some((*r).clone()),
            FnArg::Typed(pat_typed) => {
                for call_back_args in &callback_args {
                    for call_back_arg in &callback_args.args {}
                }
            }
        }
    }
    Ok(BindgenMethod {
        non_bindgen_attrs,
        callback_args,
        callback_args_vec,
        serializer,
        is_init,
        regular_args,
        is_public,
        returns,
        receiver,
    })
}

pub fn _process_method(
    method: &ImplItemMethod,
    impl_type: &Type,
    is_trait_impl: bool,
) -> syn::Result<TokenStream2> {
    let bindgen_info = _extract_bindgen_info_from_method(method, impl_type, is_trait_impl)?;
    let method_name = method.sig.ident.clone();

    let mut needs_env_init = false;

    let env_init = if needs_env_init {
        quote! {
        near_bindgen::env::set_blockchain_interface(Box::new(near_blockchain::NearBlockchain {}));
        }
    } else {
        TokenStream2::new()
    };

    Ok(quote! {
        #[cfg(target_arch = "wasm32")]
        #[no_mangle]
        #non_bindgen_attrs
        pub extern "C" fn #method_name() {
            #env_init
            #input_struct
            #arg_deconstruct
        }
    })
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

    let (arg_parsing_code, arg_list) = arg_deser::get_arg_parsing(method)?;
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
                near_bindgen::env::set_blockchain_interface(Box::new(near_blockchain::NearBlockchain {}));
                let input = near_bindgen::env::input().unwrap();
                #[derive(serde::Deserialize)]
                struct Args {
                    k: u64,
                    m: Bar
                }
                let {k, m}: Args = serde_json::from_slice(&input).unwrap();
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
