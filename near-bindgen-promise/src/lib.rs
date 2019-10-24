#![recursion_limit = "128"]
use inflector::Inflector;
use quote::quote;
use syn::export::{Span, TokenStream2};
use syn::spanned::Spanned;
use syn::{Error, FnArg, Ident, ItemTrait, LitByteStr, LitStr, Pat, TraitItem};

pub fn process_trait(t: &ItemTrait, mod_name: Option<String>) -> syn::Result<TokenStream2> {
    let mod_name = mod_name.unwrap_or_else(|| t.ident.to_string().to_snake_case());
    let mod_name = Ident::new(&mod_name, Span::call_site());
    let mut res = TokenStream2::new();
    for item in &t.items {
        match item {
            TraitItem::Type(_) => {
                return Err(Error::new(
                    item.span(),
                    "Traits for external contracts do not support associated trait types yet.",
                ))
            }
            TraitItem::Method(method) => {
                if method.default.is_some() {
                    return Err(Error::new(
                        method.span(),
                        "Traits that are used to describe external contract should not include\
                         default implementations because this is not a valid use case of traits\
                         to describe external contracts.",
                    ));
                }
                let sig = &method.sig;
                if sig.unsafety.is_some() {
                    return Err(Error::new(
                        method.span(),
                        "Methods of external contracts are not allowed to be unsafe.",
                    ));
                }
                if sig.asyncness.is_some() {
                    return Err(Error::new(
                        method.span(),
                        "Methods of external contracts are not allowed to be async.",
                    ));
                }
                if sig.abi.is_some() {
                    return Err(Error::new(
                        method.span(),
                        "Methods of external contracts are not allowed to have binary interface.",
                    ));
                }
                if !sig.generics.params.is_empty() {
                    return Err(Error::new(
                        method.span(),
                        "Methods of external contracts are not allowed to have generics.",
                    ));
                }
                if sig.variadic.is_some() {
                    return Err(Error::new(
                        method.span(),
                        "Methods of external contracts are not allowed to have variadic arguments.",
                    ));
                }

                let method_name = &sig.ident;
                let inputs = &sig.inputs;

                let method_name_byte_str =
                    LitByteStr::new(sig.ident.to_string().as_bytes(), Span::call_site());

                let mut arg_to_json = TokenStream2::new();
                let mut method_args = TokenStream2::new();
                let num_args = inputs.len();
                for (i, arg) in inputs.iter().enumerate() {
                    match arg {
                        FnArg::Receiver(_) => {}
                        FnArg::Typed(arg) => {
                            let arg_name = if let Pat::Ident(arg_name) = arg.pat.as_ref() {
                                &arg_name.ident
                            } else {
                                return Err(Error::new(
                                    arg.span(),
                                    "Unsupported argument name pattern.",
                                ));
                            };
                            let arg_name_quoted =
                                LitStr::new(&arg_name.to_string(), Span::call_site());
                            arg_to_json.extend(quote! {
                                #arg_name_quoted: #arg_name
                            });
                            method_args.extend(quote! {
                                #arg ,
                            });
                            if i < num_args - 1 {
                                arg_to_json.extend(quote! {,});
                            }
                        }
                    }
                }
                if arg_to_json.is_empty() {
                    arg_to_json = quote! {vec![]};
                } else {
                    arg_to_json = quote! {json!({ #arg_to_json }).to_string().as_bytes().to_vec()};
                }
                res.extend( quote! {
                    pub fn #method_name<T: ToString>(#method_args __account_id: &T, __balance: Balance, __gas: Gas) -> Promise {
                        Promise::new(__account_id.to_string())
                        .function_call(
                            #method_name_byte_str.to_vec(),
                            #arg_to_json,
                            __balance,
                            __gas,
                        )
                    }
                });
            }
            _ => (),
        }
    }

    Ok(quote! {
        pub mod #mod_name {
            use super::*;
            use near_bindgen::{Gas, Balance, AccountId, Promise};
            use std::string::ToString;
            #res
        }
    })
}

// Rustfmt removes comas.
#[rustfmt::skip]
#[cfg(test)]
mod tests {
    use syn::ItemTrait;
    use quote::quote;
    use crate::process_trait;

    #[test]
    fn standard() {
        let t: ItemTrait = syn::parse2(
            quote!{
                pub trait ExternalCrossContract {
                    fn merge_sort(&self, arr: Vec<u8>) -> Vec<u8>;
                    fn merge(&self) -> Vec<u8>;
                }
            }
        ).unwrap();
        
        let actual = process_trait(&t, None).unwrap();
        let expected = quote! {
            pub mod external_cross_contract {
                use super::*;
                use near_bindgen::{Gas, Balance, AccountId, Promise};
                use std::string::ToString;
                
                pub fn merge_sort<T: ToString>(arr: Vec<u8>, __account_id: &T, __balance: Balance, __gas: Gas) -> Promise {
                    Promise::new(__account_id.to_string())
                        .function_call(
                        b"merge_sort".to_vec(),
                        json!({ "arr": arr }).to_string().as_bytes().to_vec(),
                        __balance,
                        __gas,
                    )
                }

                pub fn merge<T: ToString>(__account_id: &T, __balance: Balance, __gas: Gas) -> Promise {
                    Promise::new(__account_id.to_string())
                        .function_call(
                        b"merge".to_vec(),
                        vec![],
                        __balance,
                        __gas,
                    )
                }
            }
        };
        assert_eq!(actual.to_string(), expected.to_string());
    }
}
