use super::{InitAttr, MethodKind, ReturnKind, SerializerAttr};
use crate::core_impl::utils;
use syn::{spanned::Spanned, Attribute, Receiver, ReturnType, Type};

mod call_visitor;
pub use call_visitor::CallVisitor;

mod init_visitor;
pub use init_visitor::InitVisitor;

mod view_visitor;
pub use view_visitor::ViewVisitor;

/// Traversal abstraction to walk a method declaration and build it's respective [MethodKind].
pub trait BindgenVisitor {
    fn visit_init_attr(&mut self, attr: &Attribute, init_attr: &InitAttr) -> syn::Result<()>;
    fn visit_payable_attr(&mut self, attr: &Attribute) -> syn::Result<()>;
    fn visit_private_attr(&mut self, attr: &Attribute) -> syn::Result<()>;
    fn visit_result_serializer_attr(
        &mut self,
        attr: &Attribute,
        result_serializer_attr: &SerializerAttr,
    ) -> syn::Result<()>;
    fn visit_receiver(&mut self, receiver: &Receiver) -> syn::Result<()>;
    fn visit_result(&mut self, handle_result: bool, return_type: &ReturnType) -> syn::Result<()>;

    fn build(&self) -> syn::Result<MethodKind>;
}

fn parse_return_kind(typ: &Type, handles_result: bool) -> syn::Result<ReturnKind> {
    if handles_result {
        if let Some(ok_type) = utils::extract_ok_type(typ) {
            Ok(ReturnKind::HandlesResult { ok_type: ok_type.clone() })
        } else {
            Err(syn::Error::new(
                typ.span(),
                "Function marked with #[handle_result] should return Result<T, E> (where E implements FunctionError).",
            ))
        }
    } else if utils::type_is_result(typ) {
        Err(syn::Error::new(
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
