use crate::info_extractor::serializer_attr::SerializerAttr;
use crate::info_extractor::SerializerType;
use quote::ToTokens;
use syn::export::Span;
use syn::{Attribute, Error, Ident, Pat, PatType, Token, Type};

pub enum BindgenArgType {
    /// Argument that we read from `env::input()`.
    Regular,
    /// An argument that we read from a single `env::promise_result()`.
    CallbackArg,
    /// An argument that we read from all `env::promise_result()`.
    CallbackArgVec,
}

/// A single argument of a function after it was processed by the bindgen.
pub struct ArgInfo {
    /// Attributes not related to bindgen.
    pub non_bindgen_attrs: Vec<Attribute>,
    /// The `binding` part of `ref mut binding @ SUBPATTERN: TYPE` argument.
    pub ident: Ident,
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
    /// The original `PatType` of the argument.
    pub original: PatType,
}

impl ArgInfo {
    /// Extract near-bindgen specific argument info.
    pub fn new(original: PatType) -> syn::Result<Self> {
        let mut non_bindgen_attrs = vec![];
        let ident = match original.pat.as_ref() {
            Pat::Ident(pat_ident) => pat_ident.ident.clone(),
            _ => {
                return Err(Error::new(
                    Span::call_site(),
                    format!("Only identity patterns are supported in function arguments."),
                ));
            }
        };
        let (reference, mutability, ty) = match original.ty.as_ref() {
            x @ Type::Array(_) | x @ Type::Path(_) | x @ Type::Tuple(_) => {
                (None, None, (*x).clone())
            }
            Type::Reference(r) => {
                (Some(r.and_token.clone()), r.mutability.clone(), (*r.elem.as_ref()).clone())
            }
            _ => return Err(Error::new(Span::call_site(), format!("Unsupported argument type."))),
        };
        // In the absence of callback attributes this is a regular argument.
        let mut bindgen_ty = BindgenArgType::Regular;
        // In the absence of serialization attributes this is a JSON serialization.
        let mut serializer_ty = SerializerType::JSON;
        for attr in &original.attrs {
            let attr_str = attr.path.to_token_stream().to_string();
            match attr_str.as_str() {
                "callback" => {
                    bindgen_ty = BindgenArgType::CallbackArg;
                }
                "callback_vec" => {
                    bindgen_ty = BindgenArgType::CallbackArgVec;
                }
                "serializer" => {
                    let serializer: SerializerAttr = syn::parse2(attr.tokens.clone())?;
                    serializer_ty = serializer.serializer_type;
                }
                _ => {
                    non_bindgen_attrs.push((*attr).clone());
                }
            }
        }

        Ok(Self {
            non_bindgen_attrs,
            ident,
            reference,
            mutability,
            ty,
            bindgen_ty,
            serializer_ty,
            original,
        })
    }
}
