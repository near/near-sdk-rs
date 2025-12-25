use crate::core_impl::info_extractor::SerializerType;
use crate::core_impl::utils;
use proc_macro2::{Span, TokenStream};
use quote::ToTokens;
use syn::{parse_quote, Error, Expr, Ident, Pat, PatType, Token, Type};

pub enum BindgenArgType {
    /// Argument that we read from `env::input()`.
    Regular,
    /// An argument that we read from one or many `env::promise_result()`.
    Callback { ty: CallbackBindgenArgType, max_bytes: Expr },
}

/// An argument that we read from one or many `env::promise_result()`.
pub enum CallbackBindgenArgType {
    /// An argument that we read from a single `env::promise_result()`.
    Arg,
    /// An argument that we read from a single `env::promise_result()` which handles the error.
    ResultArg,
    /// An argument that we read from all `env::promise_result()`.
    ArgVec,
}

/// A single argument of a function after it was processed by the bindgen.
pub struct ArgInfo {
    /// The `binding` part of `ref mut binding @ SUBPATTERN: TYPE` argument.
    pub ident: Ident,
    /// Whether pattern has a preceded `ref`.
    #[allow(unused)]
    pub pat_reference: Option<Token![ref]>,
    /// Whether pattern has a preceded `mut`.
    #[allow(unused)]
    pub pat_mutability: Option<Token![mut]>,
    /// Whether the `TYPE` starts with `&`.
    pub reference: Option<Token![&]>,
    /// Whether `TYPE` starts with `&mut`. Can only be set together with the `reference`.
    pub mutability: Option<Token![mut]>,
    /// The `TYPE` stripped of `&` and `mut`.
    pub ty: Type,
    /// Bindgen classification of argument type, based on what attributes it has.
    pub bindgen_ty: BindgenArgType,
    /// Type of serializer that we use for this argument.
    pub serializer_ty: SerializerType,
    /// Spans of all occurrences of the `Self` token, if any.
    pub self_occurrences: Vec<Span>,
    /// The original `PatType` of the argument.
    pub original: PatType,
}
use darling::FromAttributes;
#[derive(darling::FromAttributes, Clone, Debug)]
#[darling(attributes(serializer))]
struct SerializerAttrConfig {
    borsh: Option<bool>,
    json: Option<bool>,
}

#[derive(darling::FromAttributes, Clone, Debug)]
#[darling(attributes(callback, callback_unwrap, callback_result, callback_vec))]
struct CallbackAttrConfig {
    #[darling(default)]
    max_bytes: Option<Expr>,
}

impl ArgInfo {
    /// Extract near-sdk specific argument info.
    pub fn new(original: &mut PatType, source_type: &TokenStream) -> syn::Result<Self> {
        let pat_info = match original.pat.as_ref() {
            Pat::Ident(pat_ident) => {
                Ok((pat_ident.by_ref, pat_ident.mutability, pat_ident.ident.clone()))
            }
            _ => Err(Error::new_spanned(
                &original.pat,
                "Only identity patterns are supported in function arguments.",
            )),
        };

        let result_sanitize_and_ty = (|| {
            let sanitize_self = utils::sanitize_self(&original.ty, source_type)?;
            *original.ty.as_mut() = sanitize_self.ty.clone();
            let ty_info = utils::extract_ref_mut(original.ty.as_ref())?;
            Ok((sanitize_self, ty_info))
        })();

        // In the absence of callback attributes this is a regular argument.
        let mut bindgen_ty = BindgenArgType::Regular;
        // In the absence of serialization attributes this is a JSON serialization.
        let mut serializer_ty = SerializerType::JSON;
        let mut more_errors: Vec<Error> = Vec::new();

        original.attrs.retain(|attr| {
            let attr_str = attr.path().to_token_stream().to_string();
            match attr_str.as_str() {
                callback if callback.starts_with("callback") => {
                    let cb_bindgen_ty = match callback {
                        "callback" | "callback_unwrap" => CallbackBindgenArgType::Arg,
                        "callback_result" => CallbackBindgenArgType::ResultArg,
                        "callback_vec" => CallbackBindgenArgType::ArgVec,
                        _ => return true,
                    };
                    match CallbackAttrConfig::from_attributes(&[attr.clone()]) {
                        Ok(args) => {
                            bindgen_ty = BindgenArgType::Callback {
                                ty: cb_bindgen_ty,
                                max_bytes: args.max_bytes.unwrap_or(parse_quote!(usize::MAX)),
                            };
                        }
                        Err(e) => more_errors.push(Error::new(e.span(), e.to_string())),
                    };
                    false
                }
                "serializer" => {
                    match SerializerAttrConfig::from_attributes(&[attr.clone()]) {
                        Ok(args) => {
                            if args.borsh.is_some() && args.json.is_some() {
                                let spanned_error = syn::Error::new_spanned(
                                    attr,
                                    "Only one of `borsh` or `json` can be specified.",
                                );
                                more_errors.push(spanned_error);
                            };

                            if let Some(borsh) = args.borsh {
                                if borsh {
                                    serializer_ty = SerializerType::Borsh;
                                }
                            }
                            if let Some(json) = args.json {
                                if json {
                                    serializer_ty = SerializerType::JSON;
                                }
                            }
                        }
                        Err(e) => more_errors.push(Error::new(e.span(), e.to_string())),
                    };
                    false
                }
                _ => true,
            }
        });

        match (&pat_info, &result_sanitize_and_ty, more_errors.is_empty()) {
            (
                Ok((pat_reference, pat_mutability, ident)),
                Ok((sanitize_self, (reference, mutability, ty))),
                true,
            ) => Ok(Self {
                ident: ident.clone(),
                pat_reference: *pat_reference,
                pat_mutability: *pat_mutability,
                reference: *reference,
                mutability: *mutability,
                ty: ty.clone(),
                bindgen_ty,
                serializer_ty,
                self_occurrences: sanitize_self.self_occurrences.clone(),
                original: original.clone(),
            }),
            _ => {
                more_errors.extend(pat_info.err());
                more_errors.extend(result_sanitize_and_ty.err());
                Err(Self::combine_errors(more_errors).unwrap())
            }
        }
    }

    // helper function
    fn combine_errors(errors: impl IntoIterator<Item = Error>) -> Option<Error> {
        errors.into_iter().reduce(|mut acc, e| {
            acc.combine(syn::Error::new(e.span(), e.to_string()));
            acc
        })
    }
}
