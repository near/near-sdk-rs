use proc_macro2::Ident;
use syn::export::{Span, ToTokens};
use syn::{
    Attribute, Error, FnArg, ImplItemMethod, Pat, PatType, Receiver, ReturnType, Token, Type,
    Visibility,
};

mod serializer_attr;
use serializer_attr::SerializerAttr;

/// Type of serialization we use.
#[derive(PartialEq, Eq)]
pub enum SerializerType {
    JSON,
    Borsh,
}

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

pub struct MethodInfo {
    /// The name of the method.
    pub ident: Ident,
    /// Attributes not related to bindgen.
    pub non_bindgen_attrs: Vec<Attribute>,
    /// All arguments of the method.
    pub args: Vec<ArgInfo>,
    /// Whether method can be used as initializer.
    pub is_init: bool,
    /// The serializer that we use for `env::input()`.
    pub input_serializer: SerializerType,
    /// The serializer that we use for the return type.
    pub result_serializer: SerializerType,
    /// The receiver, like `mut self`, `self`, `&mut self`, `&self`, or `None`.
    pub receiver: Option<Receiver>,
    /// Whether method has `pub` modifier or a part of trait implementation.
    pub is_public: bool,
    /// What this function returns.
    pub returns: ReturnType,
    /// The original code of the method.
    pub original: ImplItemMethod,
    /// The type of the contract struct.
    pub struct_type: Type,
}

impl MethodInfo {
    /// Process the method and extract information important for near-bindgen.
    pub fn new(
        original: ImplItemMethod,
        struct_type: Type,
        is_trait_impl: bool,
    ) -> syn::Result<Self> {
        let ident = original.sig.ident.clone();
        let mut non_bindgen_attrs = vec![];
        let mut args = vec![];
        let mut is_init = false;
        // By the default we serialize the result with JSON.
        let mut result_serializer = SerializerType::JSON;
        for attr in &original.attrs {
            let attr_str = attr.path.to_token_stream().to_string();
            match attr_str.as_str() {
                "init" => {
                    is_init = true;
                }
                "result_serializer" => {
                    let serializer: SerializerAttr = syn::parse2(attr.tokens.clone())?;
                    result_serializer = serializer.serializer_type;
                }
                _ => non_bindgen_attrs.push((*attr).clone()),
            }
        }

        let is_public = match original.vis {
            Visibility::Public(_) => true,
            _ => is_trait_impl,
        };
        let returns = original.sig.output.clone();
        let mut receiver = None;
        for fn_arg in &original.sig.inputs {
            match fn_arg {
                FnArg::Receiver(r) => receiver = Some((*r).clone()),
                FnArg::Typed(pat_typed) => {
                    args.push(ArgInfo::new((*pat_typed).clone())?);
                }
            }
        }

        let mut result = Self {
            ident,
            non_bindgen_attrs,
            args,
            input_serializer: SerializerType::JSON,
            is_init,
            result_serializer,
            receiver,
            is_public,
            returns,
            original,
            struct_type,
        };

        let input_serializer =
            if result.input_args().all(|arg: &ArgInfo| arg.serializer_ty == SerializerType::JSON) {
                SerializerType::JSON
            } else if result.input_args().all(|arg| arg.serializer_ty == SerializerType::Borsh) {
                SerializerType::Borsh
            } else {
                return Err(Error::new(
                    Span::call_site(),
                    format!("Input arguments should be all of the same serialization type."),
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
