use crate::info_extractor::arg_info::{ArgInfo, BindgenArgType};
use crate::info_extractor::serializer_attr::SerializerAttr;
use crate::info_extractor::SerializerType;
use quote::ToTokens;
use syn::export::Span;
use syn::spanned::Spanned;
use syn::{Attribute, Error, FnArg, Ident, Receiver, ReturnType, Signature};

/// Information extracted from method attributes and signature.
pub struct AttrSigInfo {
    /// The name of the method.
    pub ident: Ident,
    /// Attributes not related to bindgen.
    pub non_bindgen_attrs: Vec<Attribute>,
    /// All arguments of the method.
    pub args: Vec<ArgInfo>,
    /// Whether method can be used as initializer.
    pub is_init: bool,
    /// Whether method accepting $NEAR.
    pub is_payable: bool,
    /// Whether method can accept calls from self (current account)
    pub is_private: bool,
    /// The serializer that we use for `env::input()`.
    pub input_serializer: SerializerType,
    /// Whether the method doesn't mutate state
    pub is_view: bool,
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

        let ident = original_sig.ident.clone();
        let mut non_bindgen_attrs = vec![];
        let mut args = vec![];
        let mut is_init = false;
        let mut is_payable = false;
        let mut is_private = false;
        // By the default we serialize the result with JSON.
        let mut result_serializer = SerializerType::JSON;

        let mut payable_attr = None;
        for attr in original_attrs.iter() {
            let attr_str = attr.path.to_token_stream().to_string();
            match attr_str.as_str() {
                "init" => {
                    is_init = true;
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
                    args.push(ArgInfo::new(pat_typed)?);
                }
            }
        }

        let is_view = if let Some(ref receiver) = receiver {
            receiver.mutability.is_none()
        } else {
            !is_init
        };

        if let Some(payable_attr) = payable_attr {
            if is_view {
                return Err(Error::new(
                    payable_attr.span(),
                    "Payable method must be mutable (not view)",
                ));
            }
        }

        original_attrs.retain(|attr| {
            let attr_str = attr.path.to_token_stream().to_string();
            attr_str != "init"
                && attr_str != "result_serializer"
                && attr_str != "payable"
                && attr_str != "private"
        });

        let returns = original_sig.output.clone();

        let mut result = Self {
            ident,
            non_bindgen_attrs,
            args,
            input_serializer: SerializerType::JSON,
            is_init,
            is_payable,
            is_private,
            is_view,
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
        self.args.iter().filter(|arg| match arg.bindgen_ty {
            BindgenArgType::Regular => true,
            _ => false,
        })
    }
}
