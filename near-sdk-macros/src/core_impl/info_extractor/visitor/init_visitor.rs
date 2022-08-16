use crate::core_impl::{InitMethod, MethodKind, Returns};

use super::{BindgenVisitor, InitAttr, SerializerAttr};
use syn::{spanned::Spanned, Attribute, Error, Receiver, ReturnType};

#[derive(Default)]
pub struct InitVisitor {
    is_payable: bool,
    ignores_state: bool,
    returns: Option<Returns>,
}

impl BindgenVisitor for InitVisitor {
    fn visit_init_attr(&mut self, _attr: &Attribute, init_attr: &InitAttr) -> syn::Result<()> {
        self.ignores_state = init_attr.ignore_state;
        Ok(())
    }

    fn visit_payable_attr(&mut self, _attr: &Attribute) -> syn::Result<()> {
        self.is_payable = true;
        Ok(())
    }

    fn visit_private_attr(&mut self, attr: &Attribute) -> syn::Result<()> {
        Err(Error::new(attr.span(), "Init function can't be private."))
    }

    fn visit_result_serializer_attr(
        &mut self,
        attr: &Attribute,
        _result_serializer_attr: &SerializerAttr,
    ) -> syn::Result<()> {
        Err(Error::new(attr.span(), "Init function can't serialize return type."))
    }

    fn visit_receiver(&mut self, receiver: &Receiver) -> syn::Result<()> {
        Err(Error::new(receiver.span(), "Init function can't have `self` parameter."))
    }

    fn visit_result(&mut self, handle_result: bool, return_type: &ReturnType) -> syn::Result<()> {
        self.returns = match return_type {
            ReturnType::Default => {
                return Err(syn::Error::new(
                    return_type.span(),
                    "Init function must return the contract state.",
                ))
            }
            ReturnType::Type(_, typ) => Some(Returns {
                original: return_type.clone(),
                kind: super::parse_return_kind(typ, handle_result)?,
            }),
        };
        Ok(())
    }

    fn build(&self) -> syn::Result<MethodKind> {
        Ok(MethodKind::Init(InitMethod {
            is_payable: self.is_payable,
            ignores_state: self.ignores_state,
            returns: self
                .returns
                .clone()
                .expect("Expected `visit_result` to be called at least once."),
        }))
    }
}
