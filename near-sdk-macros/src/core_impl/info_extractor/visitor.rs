use super::{InitAttr, MethodKind, ReturnKind, SerializerAttr};
use crate::core_impl::{utils, CallMethod, InitMethod, Returns, SerializerType, ViewMethod};
use quote::ToTokens;
use syn::{spanned::Spanned, Attribute, Error, FnArg, Receiver, ReturnType, Signature, Type};

/// Traversal abstraction to walk a method declaration and build it's respective [MethodKind].
pub struct Visitor {
    kind: VisitorKind,
    handles_result: bool,
    is_payable: bool,
    is_private: bool,
    ignores_state: bool,
    result_serializer: SerializerType,
    returns: Option<Returns>,
    receiver: Option<Receiver>,
}

#[derive(Debug, strum_macros::Display)]
enum VisitorKind {
    Call,
    View,
    Init,
}

impl Visitor {
    pub fn new(original_attrs: &[Attribute], original_sig: &Signature) -> Self {
        use VisitorKind::*;

        let kind = if is_init(original_attrs) {
            Init
        } else if is_view(original_sig) {
            View
        } else {
            Call
        };

        Self {
            kind,
            handles_result: Default::default(),
            is_payable: Default::default(),
            is_private: Default::default(),
            ignores_state: Default::default(),
            result_serializer: SerializerType::JSON,
            returns: Default::default(),
            receiver: Default::default(),
        }
    }

    pub fn visit_init_attr(&mut self, attr: &Attribute, init_attr: &InitAttr) -> syn::Result<()> {
        use VisitorKind::*;

        match self.kind {
            Init => {
                self.ignores_state = init_attr.ignore_state;
                Ok(())
            }
            Call | View => {
                let message =
                    format!("{} function can't be an init function at the same time.", self.kind);
                Err(Error::new(attr.span(), message))
            }
        }
    }

    pub fn visit_payable_attr(&mut self, attr: &Attribute) -> syn::Result<()> {
        use VisitorKind::*;

        match self.kind {
            Call | Init => {
                self.is_payable = true;
                Ok(())
            }
            View => {
                let message = format!("{} function can't be payable.", self.kind);
                Err(Error::new(attr.span(), message))
            }
        }
    }

    pub fn visit_private_attr(&mut self, attr: &Attribute) -> syn::Result<()> {
        use VisitorKind::*;

        match self.kind {
            Call | View => {
                self.is_private = true;
                Ok(())
            }
            Init => {
                let message = format!("{} function can't be private.", self.kind);
                Err(Error::new(attr.span(), message))
            }
        }
    }

    pub fn visit_result_serializer_attr(
        &mut self,
        attr: &Attribute,
        result_serializer_attr: &SerializerAttr,
    ) -> syn::Result<()> {
        use VisitorKind::*;

        match self.kind {
            Call | View => {
                self.result_serializer = result_serializer_attr.serializer_type.clone();
                Ok(())
            }
            Init => {
                let message = format!("{} function can't serialize return type.", self.kind);
                Err(Error::new(attr.span(), message))
            }
        }
    }

    pub fn visit_receiver(&mut self, receiver: &Receiver) -> syn::Result<()> {
        use VisitorKind::*;

        match self.kind {
            Call | View => {
                self.receiver = Some(receiver.clone());
                Ok(())
            }
            Init => {
                let message = format!("{} function can't have `self` parameter.", self.kind);
                Err(Error::new(receiver.span(), message))
            }
        }
    }

    pub fn visit_handles_result(&mut self) {
        self.handles_result = true
    }

    pub fn visit_return_type(&mut self, return_type: &ReturnType) -> syn::Result<()> {
        use VisitorKind::*;

        self.returns = match return_type {
            ReturnType::Default => match self.kind {
                Call | View => {
                    Some(Returns { original: return_type.clone(), kind: ReturnKind::Default })
                }
                Init => {
                    let message = format!("{} function must return the contract state.", self.kind);
                    return Err(Error::new(return_type.span(), message));
                }
            },
            ReturnType::Type(_, typ) => Some(Returns {
                original: return_type.clone(),
                kind: parse_return_kind(typ, self.handles_result)?,
            }),
        };
        Ok(())
    }

    pub fn build(self) -> MethodKind {
        use VisitorKind::*;

        let Visitor {
            kind,
            is_payable,
            is_private,
            ignores_state,
            result_serializer,
            returns,
            receiver,
            ..
        } = self;

        let returns = returns.expect("Expected `visit_result` to be called at least once.");

        match kind {
            Call => MethodKind::Call(CallMethod {
                is_payable,
                is_private,
                result_serializer,
                returns,
                receiver,
            }),
            Init => MethodKind::Init(InitMethod { is_payable, ignores_state, returns }),
            View => {
                MethodKind::View(ViewMethod { is_private, result_serializer, receiver, returns })
            }
        }
    }
}

fn is_init(attributes: &[Attribute]) -> bool {
    attributes.iter().any(|a| a.path.to_token_stream().to_string() == "init")
}

fn is_view(sig: &Signature) -> bool {
    let receiver_opt = sig.inputs.iter().find_map(|arg| match arg {
        FnArg::Receiver(r) => Some(r),
        _ => None,
    });

    match receiver_opt {
        Some(receiver) => receiver.reference.is_none() || receiver.mutability.is_none(),
        None => true,
    }
}

fn parse_return_kind(typ: &Type, handles_result: bool) -> syn::Result<ReturnKind> {
    if handles_result {
        if let Some(ok_type) = utils::extract_ok_type(typ) {
            Ok(ReturnKind::HandlesResult { ok_type: ok_type.clone() })
        } else {
            Err(Error::new(
                typ.span(),
                "Function marked with #[handle_result] should return Result<T, E> (where E implements FunctionError).",
            ))
        }
    } else if utils::type_is_result(typ) {
        Err(Error::new(
            typ.span(),
            "Serializing Result<T, E> has been deprecated. Consider marking your method \
                with #[handle_result] if the second generic represents a panicable error or \
                replacing Result with another two type sum enum otherwise. If you really want \
                to keep the legacy behavior, mark the method with #[handle_result] and make \
                it return Result<Result<T, E>, near_sdk::Abort>.",
        ))
    } else {
        Ok(ReturnKind::General(typ.clone()))
    }
}
