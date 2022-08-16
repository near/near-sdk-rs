use super::visitor::{BindgenVisitor, CallVisitor, InitVisitor, ViewVisitor};
use super::{
    ArgInfo, BindgenArgType, InitAttr, MethodKind, MethodType, ReturnKind, SerializerAttr,
    SerializerType,
};
use proc_macro2::Span;
use quote::ToTokens;
use syn::spanned::Spanned;
use syn::{Attribute, Error, FnArg, Ident, Receiver, ReturnType, Signature};

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
    pub fn new(
        original_attrs: &mut Vec<Attribute>,
        original_sig: &mut Signature,
    ) -> syn::Result<Self> {
        if original_sig.asyncness.is_some() {
            return Err(Error::new(
                original_sig.span(),
                "Contract API is not allowed to be async.",
            ));
        }
        if original_sig.abi.is_some() {
            return Err(Error::new(
                original_sig.span(),
                "Contract API is not allowed to have binary interface.",
            ));
        }
        if original_sig.variadic.is_some() {
            return Err(Error::new(
                original_sig.span(),
                "Contract API is not allowed to have variadic arguments.",
            ));
        }

        // Run early checks to determine the method type
        let mut visitor: Box<dyn BindgenVisitor> = if original_attrs
            .iter()
            .any(|a| a.path.to_token_stream().to_string() == "init")
        {
            Box::new(InitVisitor::default())
        } else if original_sig.inputs.iter().any(
            |i| matches!(i, FnArg::Receiver(r) if r.reference.is_none() || r.mutability.is_none()),
        ) {
            Box::new(ViewVisitor::default())
        } else {
            Box::new(CallVisitor::default())
        };

        let ident = original_sig.ident.clone();
        let mut non_bindgen_attrs = vec![];
        let mut handles_result = false;
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
                    handles_result = true;
                }
                _ => {
                    non_bindgen_attrs.push((*attr).clone());
                }
            }
        }

        let mut args = vec![];
        for fn_arg in &mut original_sig.inputs {
            match fn_arg {
                FnArg::Receiver(r) => visitor.visit_receiver(r)?,
                FnArg::Typed(pat_typed) => {
                    args.push(ArgInfo::new(pat_typed)?);
                }
            }
        }

        visitor.visit_result(handles_result, &original_sig.output)?;
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
