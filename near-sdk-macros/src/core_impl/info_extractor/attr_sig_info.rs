use super::{ArgInfo, BindgenArgType, InitAttr, MethodType, SerializerAttr, SerializerType};
use crate::core_impl::utils;
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::ToTokens;
use syn::spanned::Spanned;
use syn::{Attribute, Error, FnArg, GenericParam, Ident, Receiver, ReturnType, Signature};

/// Information extracted from method attributes and signature.
pub struct AttrSigInfo {
    /// The name of the method.
    pub ident: Ident,
    /// Attributes not related to bindgen.
    pub non_bindgen_attrs: Vec<Attribute>,
    /// All arguments of the method.
    pub args: Vec<ArgInfo>,
    /// Describes the type of the method.
    pub method_type: MethodType,
    /// Whether method accepting $NEAR.
    pub is_payable: bool,
    /// Whether method can accept calls from self (current account)
    pub is_private: bool,
    /// Whether method returns Result type where only Ok type is serialized
    pub is_handles_result: bool,
    /// The serializer that we use for `env::input()`.
    pub input_serializer: SerializerType,
    /// The serializer that we use for the return type.
    pub result_serializer: SerializerType,
    /// The receiver, like `mut self`, `self`, `&mut self`, `&self`, or `None`.
    pub receiver: Option<Receiver>,
    /// What this function returns.
    pub returns: ReturnType,
    /// The original method signature.
    pub original_sig: Signature,
}

impl AttrSigInfo {
    /// Process the method and extract information important for near-sdk.
    pub fn new(
        original_attrs: &mut Vec<Attribute>,
        original_sig: &mut Signature,
        source_type: &TokenStream2,
    ) -> syn::Result<Self> {
        let mut errors = vec![];
        for generic in &original_sig.generics.params {
            match generic {
                GenericParam::Type(type_generic) => {
                    errors.push(Error::new(
                        type_generic.span(),
                        "Contract API is not allowed to have generics.",
                    ));
                }
                GenericParam::Const(const_generic) => {
                    // `generic.span()` points to the `const` part of const generics, so we use `ident` explicitly.
                    errors.push(Error::new(
                        const_generic.ident.span(),
                        "Contract API is not allowed to have generics.",
                    ));
                }
                _ => {}
            }
        }
        if let Some(combined_errors) = errors.into_iter().reduce(|mut l, r| (l.combine(r), l).1) {
            return Err(combined_errors);
        }
        let ident = original_sig.ident.clone();
        let mut non_bindgen_attrs = vec![];
        let mut args = vec![];
        let mut method_type = MethodType::Regular;
        let mut is_payable = false;
        let mut is_private = false;
        let mut is_handles_result = false;
        // By the default we serialize the result with JSON.
        let mut result_serializer = SerializerType::JSON;

        let mut payable_attr = None;
        for attr in original_attrs.iter() {
            let attr_str = attr.path.to_token_stream().to_string();
            match attr_str.as_str() {
                "init" => {
                    let init_attr: InitAttr = syn::parse2(attr.tokens.clone())?;
                    if init_attr.ignore_state {
                        method_type = MethodType::InitIgnoreState;
                    } else {
                        method_type = MethodType::Init;
                    }
                }
                "payable" => {
                    payable_attr = Some(attr);
                    is_payable = true;
                }
                "private" => {
                    is_private = true;
                }
                "result_serializer" => {
                    let serializer: SerializerAttr = syn::parse2(attr.tokens.clone())?;
                    result_serializer = serializer.serializer_type;
                }
                "handle_result" => {
                    is_handles_result = true;
                }
                _ => {
                    non_bindgen_attrs.push((*attr).clone());
                }
            }
        }

        let mut receiver = None;
        for fn_arg in &mut original_sig.inputs {
            match fn_arg {
                FnArg::Receiver(r) => receiver = Some((*r).clone()),
                FnArg::Typed(pat_typed) => {
                    args.push(ArgInfo::new(pat_typed, source_type)?);
                }
            }
        }

        if let Some(ref receiver) = receiver {
            if matches!(method_type, MethodType::Regular) {
                if receiver.mutability.is_none() || receiver.reference.is_none() {
                    method_type = MethodType::View;
                }
            } else {
                return Err(Error::new(
                    payable_attr.span(),
                    "Init methods can't have `self` attribute",
                ));
            }
        };

        if let Some(payable_attr) = payable_attr {
            if matches!(method_type, MethodType::View) {
                return Err(Error::new(
                    payable_attr.span(),
                    "Payable method must be mutable (not view)",
                ));
            }
        }

        *original_attrs = non_bindgen_attrs.clone();
        let mut returns = original_sig.output.clone();

        if let ReturnType::Type(_, ref mut ty) = returns {
            *ty.as_mut() = utils::sanitize_self(&*ty, source_type)?;
        }

        let mut result = Self {
            ident,
            non_bindgen_attrs,
            args,
            input_serializer: SerializerType::JSON,
            method_type,
            is_payable,
            is_private,
            is_handles_result,
            result_serializer,
            receiver,
            returns,
            original_sig: original_sig.clone(),
        };

        let input_serializer =
            if result.input_args().all(|arg: &ArgInfo| arg.serializer_ty == SerializerType::JSON) {
                SerializerType::JSON
            } else if result.input_args().all(|arg| arg.serializer_ty == SerializerType::Borsh) {
                SerializerType::Borsh
            } else {
                return Err(Error::new(
                    Span::call_site(),
                    "Input arguments should be all of the same serialization type.",
                ));
            };
        result.input_serializer = input_serializer;
        Ok(result)
    }

    /// Only get args that correspond to `env::input()`.
    pub fn input_args(&self) -> impl Iterator<Item = &ArgInfo> {
        self.args.iter().filter(|arg| matches!(arg.bindgen_ty, BindgenArgType::Regular))
    }
}
