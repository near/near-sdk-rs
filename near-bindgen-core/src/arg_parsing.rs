use syn::export::{Span, TokenStream2};
use syn::spanned::Spanned;
use syn::{Error, FnArg, ImplItemMethod, Pat, Type};

use quote::quote;
use std::collections::HashMap;
use std::iter::FromIterator;

/// Check that narrows down argument types and return type descriptive enough for deserialization and serialization.
pub fn check_arg_return_type(ty: &Type, span: Span) -> syn::Result<()> {
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
        | Type::Verbatim(_)
        | Type::__Nonexhaustive => Err(Error::new(
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
    let mut callback_args: HashMap<String, u64> = crate::callback_args::parse_args(method)?
        .map(|args| HashMap::from_iter(args.into_iter().enumerate().map(|(k, v)| (v, k as u64))))
        .unwrap_or_default();
    // If we parse callback args explicitly then we add assertion that we receive correct number of
    // argument through callback.
    if !callback_args.is_empty() {
        let num = callback_args.len() as u64;
        result.extend(quote! {
                    assert_eq!(near_bindgen::env::promise_results_count(), #num);
        });
    }
    let mut result_args = TokenStream2::new();
    for arg in &method.sig.inputs {
        match arg {
            // Allowed types of arguments.
            FnArg::Receiver(_) => {}
            FnArg::Typed(arg) => {
                let arg_name = if let Pat::Ident(arg_name) = arg.pat.as_ref() {
                    arg_name
                } else {
                    return Err(Error::new(arg.span(), "Unsupported argument name pattern."));
                };
                let arg_name_quoted = quote! { #arg_name }.to_string();

                check_arg_return_type(&arg.ty, arg.span())?;

                if let Type::Reference(r) = arg.ty.as_ref() {
                    let ty = &r.elem;
                    let mutability = &r.mutability;
                    // Depending on whether the argument was passed through callback or not we
                    // retrieve it differently.
                    if let Some(callback_arg_index) = callback_args.remove(&arg_name_quoted) {
                        result.extend(quote! {
                                let data: Vec<u8> = match near_bindgen::env::promise_result(#callback_arg_index) {
                                    near_bindgen::PromiseResult::Successful(x) => x,
                                    _ => panic!("Callback computation {} was not successful", #callback_arg_index)
                                };
                                let #mutability #arg_name: #ty = serde_json::from_slice(&data).unwrap();
                            });
                    } else {
                        result.extend(quote! {
                                let #mutability #arg_name: #ty = serde_json::from_value(args[#arg_name_quoted].clone()).unwrap();
                            });
                    }
                    result_args.extend(quote! {
                        & #mutability #arg_name ,
                    });
                } else {
                    if let Some(callback_arg_index) = callback_args.remove(&arg_name_quoted) {
                        result.extend(quote! {
                                let data: Vec<u8> = match near_bindgen::env::promise_result(#callback_arg_index) {
                                    near_bindgen::PromiseResult::Successful(x) => x,
                                    _ => panic!("Callback computation {} was not successful", #callback_arg_index)
                                };
                                let #arg = serde_json::from_slice(&data).unwrap();
                            });
                    } else {
                        result.extend(quote! {
                            let #arg = serde_json::from_value(args[#arg_name_quoted].clone()).unwrap();
                        });
                    }
                    result_args.extend(quote! {
                        #arg_name ,
                    });
                }
            }
        }
    }

    if !callback_args.is_empty() {
        return Err(Error::new(
            Span::call_site(),
            format!("callback_args(..) macro should contain args used in the method signature. Args not used: {:?}",
                callback_args.keys().cloned().collect::<Vec<_>>()
            )
        ));
    }

    // If there are some args then add parsing header.
    if !result.is_empty() {
        result = quote! {
            let args: serde_json::Value = serde_json::from_slice(&near_bindgen::env::input().unwrap()).unwrap();
            #result
        };
    }
    Ok((result, result_args))
}
