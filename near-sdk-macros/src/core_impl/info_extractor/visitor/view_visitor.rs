use super::{BindgenVisitor, InitAttr, ReturnKind, SerializerAttr};
use crate::core_impl::{MethodKind, Returns, SerializerType, ViewMethod};
use syn::{spanned::Spanned, Attribute, Error, Receiver, ReturnType};

pub struct ViewVisitor {
    is_private: bool,
    result_serializer: SerializerType,
    returns: Option<Returns>,
    receiver: Option<Receiver>,
}

impl Default for ViewVisitor {
    fn default() -> Self {
        Self {
            is_private: Default::default(),
            result_serializer: SerializerType::JSON,
            returns: Default::default(),
            receiver: Default::default(),
        }
    }
}

impl BindgenVisitor for ViewVisitor {
    fn visit_init_attr(&mut self, attr: &Attribute, _init_attr: &InitAttr) -> syn::Result<()> {
        Err(Error::new(attr.span(), "View function can't be an init function at the same time."))
    }

    fn visit_payable_attr(&mut self, attr: &Attribute) -> syn::Result<()> {
        Err(Error::new(attr.span(), "View function can't be payable."))
    }

    fn visit_private_attr(&mut self, _attr: &Attribute) -> syn::Result<()> {
        self.is_private = true;
        Ok(())
    }

    fn visit_result_serializer_attr(
        &mut self,
        _attr: &Attribute,
        result_serializer_attr: &SerializerAttr,
    ) -> syn::Result<()> {
        self.result_serializer = result_serializer_attr.serializer_type.clone();
        Ok(())
    }

    fn visit_receiver(&mut self, receiver: &Receiver) -> syn::Result<()> {
        self.receiver = Some(receiver.clone());
        Ok(())
    }

    fn visit_result(&mut self, handle_result: bool, return_type: &ReturnType) -> syn::Result<()> {
        self.returns = match return_type {
            ReturnType::Default => {
                Some(Returns { original: return_type.clone(), kind: ReturnKind::Default })
            }
            ReturnType::Type(_, typ) => Some(Returns {
                original: return_type.clone(),
                kind: super::parse_return_kind(typ, handle_result)?,
            }),
        };
        Ok(())
    }

    fn build(&self) -> syn::Result<MethodKind> {
        Ok(MethodKind::View(ViewMethod {
            is_private: self.is_private,
            result_serializer: self.result_serializer.clone(),
            returns: self
                .returns
                .clone()
                .expect("Expected `visit_result` to be called at least once."),
            receiver: self.receiver.clone(),
        }))
    }
}
