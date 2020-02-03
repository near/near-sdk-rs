use syn::export::{Span, TokenStream2};
use syn::spanned::Spanned;
use syn::{
    AttrStyle, Attribute, Error, Expr, Field, Fields, FieldsNamed, FnArg, ImplItemMethod,
    ItemStruct, Pat, PatType, Token, Type, Visibility,
};

use crate::SerializerType;
use proc_macro2::Ident;
use quote::quote;
use std::collections::HashMap;
use std::iter::FromIterator;
use syn::punctuated::Punctuated;
use syn::token::Token;

/// Separate reference and mutability from the type, if any.
fn split_ref_mut_type(ty: &Type) -> syn::Result<(Option<Token![&]>, Option<Token![mut]>, Type)> {
    match ty {
        x @ Type::Array(_) | x @ Type::Path(_) | x @ Type::Tuple(_) => {
            Ok((None, None, (*x).clone()))
        }
        Type::Reference(r) => {
            Ok((Some(r.and_token.clone()), r.mutability.clone(), (*r.elem.as_ref()).clone()))
        }
        _ => Err(Error::new(Span::call_site(), format!("Unsupported argument type."))),
    }
}

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
///   arg2: (u64, Vec<String>)
/// }
/// ```
pub fn create_input_struct<T>(
    fn_args: T,
    serializer_type: &SerializerType,
) -> syn::Result<Option<ItemStruct>>
where
    T: Iterator<Item = PatType>,
{
    let attribute = match serializer_type {
        SerializerType::JSON => quote! {#[derive(Deserialize)]},
        SerializerType::Borsh => quote! {#[derive(BorshDeserialize)]},
    };
    let mut fields = TokenStream2::new();

    for fn_arg in fn_args {
        match fn_arg.pat.as_ref() {
            Pat::Ident(pat_ident) => {
                let ident = &pat_ident.ident;
                let ty = split_ref_mut_type(fn_arg.ty.as_ref())?.2;
                fields.extend(quote! {
                    #ident: #ty,
                });
            }
            _ => {
                return Err(Error::new(
                    fn_arg.span(),
                    format!("Only identity patterns are supported in function arguments."),
                ));
            }
        }
    }
    if fields.is_empty() {
        Ok(None)
    } else {
        Ok(Some(syn::parse2(quote! {
        #attribute
        struct Input {
            #fields
        }
        })?))
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
fn create_decompose_input_pat<T>(fn_args: T) -> syn::Result<Pat>
where
    T: Iterator<Item = PatType>,
{
    let mut fields = TokenStream2::new();
    for fn_arg in fn_args {
        match fn_arg.pat.as_ref() {
            Pat::Ident(pat_ident) => {
                let ident = &pat_ident.ident;
                let (_, mutability, ty) = split_ref_mut_type(fn_arg.ty.as_ref())?;
                fields.extend(quote! {
                    #mutability #ident,
                });
            }
            _ => {
                return Err(Error::new(
                    fn_arg.span(),
                    format!("Only identity patterns are supported in function arguments."),
                ));
            }
        }
    }
    syn::parse2(quote! {
        Input {
            #fields
        }
    })
}

/// Create a sequence of arguments that can be used to call the method or the function
/// of the smart contract.
///
/// # Example:
/// ```
/// a, &b, &mut c,
/// ```
fn create_call_args<T>(fn_args: T) -> syn::Result<TokenStream2>
where
    T: Iterator<Item = PatType>,
{
    let mut result = TokenStream2::new();
    for fn_arg in fn_args {
        match fn_arg.pat.as_ref() {
            Pat::Ident(pat_ident) => {
                let ident = &pat_ident.ident;
                let (reference, mutability, ty) = split_ref_mut_type(fn_arg.ty.as_ref())?;
                result.extend(quote! {
                    #reference #mutability #ident,
                });
            }
            _ => {
                return Err(Error::new(
                    fn_arg.span(),
                    format!("Only identity patterns are supported in function arguments."),
                ));
            }
        }
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use crate::arg_deser::{
        create_call_args, create_decompose_input_pat, create_input_struct, SerializerType,
    };
    use quote::quote;
    use syn::{FnArg, ImplItemMethod, ItemStruct, Pat, Signature, Type};

    #[test]
    fn test_input_struct() {
        let method: ImplItemMethod = syn::parse2(quote! {
            fn simple_function(mut a: String, b: &'a Vec<u64>, c: & mut T) {}
        })
        .unwrap();
        let signature = method.sig;
        let actual = create_input_struct(
            signature.inputs.into_iter().map(|arg| match arg {
                FnArg::Typed(t) => t,
                _ => unimplemented!(),
            }),
            &SerializerType::Borsh,
        )
        .unwrap()
        .unwrap();
        let expected: ItemStruct = syn::parse2(quote! {
        #[derive(BorshDeserialize)]
        struct Input {
            a: String,
            b: Vec<u64>,
            c: T,
        }
        })
        .unwrap();
        assert_eq!(quote! {#actual}.to_string(), quote! {#expected}.to_string());
    }

    #[test]
    fn test_input_struct_decompose() {
        let method: ImplItemMethod = syn::parse2(quote! {
            fn simple_function(mut a: String, b: &'a Vec<u64>, c: & mut T) {}
        })
        .unwrap();
        let signature = method.sig;
        let actual =
            create_decompose_input_pat(signature.inputs.into_iter().map(|arg| match arg {
                FnArg::Typed(t) => t,
                _ => unimplemented!(),
            }))
            .unwrap();
        let expected: Pat = syn::parse2(quote! {
        Input {
            a,
            b,
            mut c,
        }
        })
        .unwrap();
        assert_eq!(quote! {#actual}.to_string(), quote! {#expected}.to_string());
    }

    #[test]
    fn test_call_args() {
        let method: ImplItemMethod = syn::parse2(quote! {
            fn simple_function(mut a: String, b: &'a Vec<u64>, c: & mut T) {}
        })
        .unwrap();
        let signature = method.sig;
        let actual = create_call_args(signature.inputs.into_iter().map(|arg| match arg {
            FnArg::Typed(t) => t,
            _ => unimplemented!(),
        }))
        .unwrap();
        let expected = quote! {
            a, &b, &mut c,
        };
        assert_eq!(actual.to_string(), expected.to_string());
    }

    //    #[test]
    //    fn standard() {
    //        let method: ImplItemMethod = syn::parse2(quote! {
    //            #[some_attribute0 tokens ? & hello]
    //            #[callback_args(arg0, arg1)]
    //            #[some_attribute1 tokens ->]
    //            fn simple_function() {
    //            }
    //        })
    //            .unwrap();
    //        let actual = parse_args(&method).unwrap();
    //        let expected = Some(vec!["arg0".to_string(), "arg1".to_string()]);
    //        assert_eq!(actual, expected);
    //    }
    //
    //    #[test]
    //    fn one_arg() {
    //        let method: ImplItemMethod = syn::parse2(quote! {
    //            #[callback_args(arg0)]
    //            fn simple_function() {
    //
    //            }
    //        })
    //            .unwrap();
    //        let actual = parse_args(&method).unwrap();
    //        let expected = Some(vec!["arg0".to_string()]);
    //        assert_eq!(actual, expected);
    //    }
}
