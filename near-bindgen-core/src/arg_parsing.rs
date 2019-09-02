use syn::export::{Span, TokenStream2};
use syn::spanned::Spanned;
use syn::{Error, FnArg, ImplItemMethod, Pat, Type};
use quote::quote;

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

                check_arg_return_type(&arg.ty, arg.span())?;

                if let Type::Reference(r) = &arg.ty {
                    let ty = &r.elem;
                    if r.mutability.is_some() {
                        result.extend(quote! {
                                let mut #arg_name: #ty = serde_json::from_value(args[#arg_name_quoted].clone()).unwrap();
                            });
                        result_args.extend(quote! {
                            &mut #arg_name ,
                        });
                    } else {
                        result.extend(quote! {
                                let #arg_name: #ty = serde_json::from_value(args[#arg_name_quoted].clone()).unwrap();
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
            _ => return Err(Error::new(arg.span(), format!("Unsupported argument type"))),
        }
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
