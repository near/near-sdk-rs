use super::visitor::Visitor;
use super::{
    ArgInfo, BindgenArgType, InitAttr, MethodKind, MethodType, ReturnKind, SerializerAttr,
    SerializerType,
};
use crate::core_impl::utils;
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::ToTokens;
use syn::spanned::Spanned;
use syn::{Attribute, Error, FnArg, GenericParam, Ident, Receiver, ReturnType, Signature, Type};

/// Information extracted from method attributes and signature.
pub struct AttrSigInfoV2 {
    /// The name of the method.
    pub ident: Ident,
    /// Attributes not related to bindgen.
    pub non_bindgen_attrs: Vec<Attribute>,
    /// All arguments of the method.
    pub args: Vec<ArgInfo>,
    /// Describes the type of the method.
    pub method_kind: MethodKind,
    /// The serializer that we use for `env::input()`.
    pub input_serializer: SerializerType,
    /// The original method signature.
    pub original_sig: Signature,
}

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

// FIXME: Remove once we switch over to AttrSigInfoV2
impl From<AttrSigInfoV2> for AttrSigInfo {
    fn from(info: AttrSigInfoV2) -> Self {
        match info.method_kind {
            MethodKind::Call(call_method) => AttrSigInfo {
                ident: info.ident,
                non_bindgen_attrs: info.non_bindgen_attrs,
                args: info.args,
                method_type: MethodType::Regular,
                is_payable: call_method.is_payable,
                is_private: call_method.is_private,
                is_handles_result: matches!(
                    call_method.returns.kind,
                    ReturnKind::HandlesResult { .. }
                ),
                input_serializer: info.input_serializer,
                result_serializer: call_method.result_serializer,
                receiver: call_method.receiver,
                returns: call_method.returns.original,
                original_sig: info.original_sig,
            },
            MethodKind::View(view_method) => AttrSigInfo {
                ident: info.ident,
                non_bindgen_attrs: info.non_bindgen_attrs,
                args: info.args,
                method_type: MethodType::View,
                is_payable: false,
                is_private: view_method.is_private,
                is_handles_result: matches!(
                    view_method.returns.kind,
                    ReturnKind::HandlesResult { .. }
                ),
                input_serializer: info.input_serializer,
                result_serializer: view_method.result_serializer,
                receiver: view_method.receiver,
                returns: view_method.returns.original,
                original_sig: info.original_sig,
            },
            MethodKind::Init(init_method) => AttrSigInfo {
                ident: info.ident,
                non_bindgen_attrs: info.non_bindgen_attrs,
                args: info.args,
                method_type: if init_method.ignores_state {
                    MethodType::InitIgnoreState
                } else {
                    MethodType::Init
                },
                is_payable: init_method.is_payable,
                is_private: false,
                is_handles_result: matches!(
                    init_method.returns.kind,
                    ReturnKind::HandlesResult { .. }
                ),
                input_serializer: info.input_serializer,
                result_serializer: SerializerType::JSON,
                receiver: None,
                returns: init_method.returns.original,
                original_sig: info.original_sig,
            },
        }
    }
}

impl AttrSigInfo {
    fn sanitize_self(original_sig: &mut Signature, source_type: &TokenStream2) -> syn::Result<()> {
        match original_sig.output {
            ReturnType::Default => {}
            ReturnType::Type(_, ref mut ty) => {
                match ty.as_mut() {
                    x @ (Type::Array(_) | Type::Path(_) | Type::Tuple(_) | Type::Group(_)) => {
                        *ty = utils::sanitize_self(x, source_type)?.into();
                    }
                    Type::Reference(ref mut r) => {
                        r.elem = utils::sanitize_self(&r.elem, source_type)?.into();
                    }
                    _ => return Err(Error::new(ty.span(), "Unsupported contract API type.")),
                };
            }
        };
        Ok(())
    }

    pub fn new(
        original_attrs: &mut Vec<Attribute>,
        original_sig: &mut Signature,
        source_type: &TokenStream2,
    ) -> syn::Result<Self> {
        Self::sanitize_self(original_sig, source_type)?;

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

        let mut visitor = Visitor::new(original_attrs, original_sig);

        let ident = original_sig.ident.clone();
        let mut non_bindgen_attrs = vec![];

        // Visit attributes
        for attr in original_attrs.iter() {
            let attr_str = attr.path.to_token_stream().to_string();
            match attr_str.as_str() {
                "init" => {
                    let init_attr: InitAttr = syn::parse2(attr.tokens.clone())?;
                    visitor.visit_init_attr(attr, &init_attr)?;
                }
                "payable" => {
                    visitor.visit_payable_attr(attr)?;
                }
                "private" => {
                    visitor.visit_private_attr(attr)?;
                }
                "result_serializer" => {
                    let serializer: SerializerAttr = syn::parse2(attr.tokens.clone())?;
                    visitor.visit_result_serializer_attr(attr, &serializer)?;
                }
                "handle_result" => {
                    visitor.visit_handle_result_attr();
                }
                _ => {
                    non_bindgen_attrs.push((*attr).clone());
                }
            }
        }

        // Visit arguments
        let mut args = vec![];
        for fn_arg in &mut original_sig.inputs {
            match fn_arg {
                FnArg::Receiver(r) => visitor.visit_receiver(r)?,
                FnArg::Typed(pat_typed) => {
                    args.push(ArgInfo::new(pat_typed, source_type)?);
                }
            }
        }

        let method_kind = visitor.build()?;

        *original_attrs = non_bindgen_attrs.clone();

        let mut result: AttrSigInfo = AttrSigInfoV2 {
            ident,
            non_bindgen_attrs,
            args,
            method_kind,
            input_serializer: SerializerType::JSON,
            original_sig: original_sig.clone(),
        }
        .into();

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
