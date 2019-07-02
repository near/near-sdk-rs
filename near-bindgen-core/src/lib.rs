#![recursion_limit="128"]
use quote::quote;
use syn::export::TokenStream2;
use syn::spanned::Spanned;
use syn::{
    FnArg, ImplItem, ImplItemMethod, ItemImpl, Pat, ReturnType, Visibility,
};

/// Attempts processing `impl` method. If method is `pub` and has `&self` or `&mut self` then it is
/// considered to be a part of contract API, otherwise `None` is returned. If method is a valid
/// contract API then we examine its arguments and fail if they use complex pattern matching.
pub fn process_method(
    method: &ImplItemMethod,
    impl_type: &syn::Type,
) -> Option<syn::Result<TokenStream2>> {
    match method.vis {
        Visibility::Public(_) => {}
        _ => return None,
    }
    if !method.sig.decl.generics.params.is_empty() {
        return Some(Err(syn::Error::new(method.sig.decl.generics.params.span(), "Methods exposed as contract API cannot use type parameters")));
    }
    // Method name.
    let method_name = &method.sig.ident;
    let mut out_args = quote! {};
    let mut method_args = quote! {};
    let mut is_mut = None;
    for arg in &method.sig.decl.inputs {
        match arg {
            FnArg::SelfRef(r) => {
                is_mut = Some(r.mutability.is_some());
            }
            FnArg::SelfValue(v) => {
                is_mut = Some(v.mutability.is_some());
            }
            FnArg::Captured(c) => {
                let ident = if let Pat::Ident(ident) = &c.pat {
                    ident
                } else {
                    return Some(Err(syn::Error::new(
                        c.span(),
                        format!("Unsupported argument type"),
                    )));
                };
                let ident_quoted = quote! { #ident }.to_string();
                let out_arg = quote! {
                    let #c = serde_json::from_value(args[#ident_quoted].clone()).unwrap();
                };
                out_args = quote! { #out_args #out_arg };
                method_args = quote! { #method_args #ident ,};
            }
            _ => {
                return Some(Err(syn::Error::new(arg.span(), format!("Unsupported argument type"))))
            }
        }
    }
    // If any args were found then add the parsing function.
    let args_parsing = if !out_args.is_empty() {
        quote! {
        let args: serde_json::Value = serde_json::from_slice(&input_read()).unwrap();
        }
    } else {
        quote! {}
    };
    let method_call = quote! {
        let mut contract: #impl_type = read_state().unwrap_or_default();
        let result = contract.#method_name(#method_args);
    };

    // Only process methods that are &self or &mut self.
    let is_mut = if let Some(is_mut) = is_mut { is_mut } else { return None };
    // If method mutates the state then record it.
    let write_state = if is_mut {
        quote! { write_state(&contract); }
    } else {
        quote! {}
    };

    // If the function returns something then return it.
    let return_value = if let ReturnType::Type(_, _) = &method.sig.decl.output {
        quote! {
         let result = serde_json::to_vec(&result).unwrap();
         unsafe {
             return_value(result.len() as _, result.as_ptr());
         }
        }
    } else {
        quote! {}
    };
    let body = quote! {
     #args_parsing
     #out_args
     #method_call
     #write_state
     #return_value
    };
    let full_method = quote! {
        #[no_mangle]
        pub extern "C" fn #method_name() {
        #body
        }
    };
    Some(Ok(TokenStream2::from(full_method)))
}

/// Processes `impl` section of the struct.
pub fn process_impl(item_impl: &ItemImpl) -> TokenStream2 {
    if !item_impl.generics.params.is_empty() {
        return syn::Error::new(item_impl.generics.params.span(), "Impl type parameters are not supported for smart contracts.").to_compile_error()
    }
    let mut output = TokenStream2::new();

    // Type for which impl is called.
    let impl_type = item_impl.self_ty.as_ref();
    for subitem in &item_impl.items {
        if let ImplItem::Method(m) = subitem {
            if let Some(wrapped_method) = process_method(m, impl_type) {
                match wrapped_method {
                    Ok(wrapped_method) => output.extend(wrapped_method),
                    Err(err) => {
                        output.extend(err.to_compile_error());
                        break;
                    }
                }
            }
        }
    }
    output
}

#[cfg(test)]
mod tests {
    use syn::{Type, ImplItemMethod};
    use quote::quote;
    use crate::process_method;

    #[test]
    fn no_args_no_return_no_mut() {
        let impl_type: Type = syn::parse_str("Hello").unwrap();
        let method: ImplItemMethod = syn::parse_str("pub fn method(&self) { }").unwrap();

        let actual = process_method(&method, &impl_type).unwrap().unwrap();
        let expected = quote!(
            #[no_mangle]
            pub extern "C" fn method() {
            let mut contract: Hello = read_state().unwrap_or_default();
            let result = contract.method();
            }
        );
        assert_eq!(expected.to_string(), actual.to_string());
    }

    #[test]
    fn no_args_no_return_mut() {
        let impl_type: Type = syn::parse_str("Hello").unwrap();
        let method: ImplItemMethod = syn::parse_str("pub fn method(&mut self) { }").unwrap();

        let actual = process_method(&method, &impl_type).unwrap().unwrap();
        let expected = quote!(
            #[no_mangle]
            pub extern "C" fn method() {
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

        let actual = process_method(&method, &impl_type).unwrap().unwrap();
        let expected = quote!(
            #[no_mangle]
            pub extern "C" fn method() {
                let args: serde_json::Value = serde_json::from_slice(&input_read()).unwrap();
                let k: u64 = serde_json::from_value(args["k"].clone()).unwrap();
                let mut contract: Hello = read_state().unwrap_or_default();
                let result = contract.method(k, );
            }
        );
        assert_eq!(expected.to_string(), actual.to_string());
    }

    #[test]
    fn args_no_return_mut() {
        let impl_type: Type = syn::parse_str("Hello").unwrap();
        let method: ImplItemMethod = syn::parse_str("pub fn method(&mut self, k: u64, m: Bar) { }").unwrap();

        let actual = process_method(&method, &impl_type).unwrap().unwrap();
        let expected = quote!(
            #[no_mangle]
            pub extern "C" fn method() {
                let args: serde_json::Value = serde_json::from_slice(&input_read()).unwrap();
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
        let method: ImplItemMethod = syn::parse_str("pub fn method(&mut self, k: u64, m: Bar) -> Option<u64> { }").unwrap();

        let actual = process_method(&method, &impl_type).unwrap().unwrap();
        let expected = quote!(
            #[no_mangle]
            pub extern "C" fn method() {
                let args: serde_json::Value = serde_json::from_slice(&input_read()).unwrap();
                let k: u64 = serde_json::from_value(args["k"].clone()).unwrap();
                let m: Bar = serde_json::from_value(args["m"].clone()).unwrap();
                let mut contract: Hello = read_state().unwrap_or_default();
                let result = contract.method(k, m, );
                write_state(&contract);
                let result = serde_json::to_vec(&result).unwrap();
                unsafe { return_value(result.len() as _ , result.as_ptr()); }
            }
        );
        assert_eq!(expected.to_string(), actual.to_string());
    }
}
