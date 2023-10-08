use crate::ImplItemMethodInfo;
use syn::spanned::Spanned;
use syn::{Error, ImplItem, ItemImpl, Type};

/// Information extracted from `impl` section.
pub struct ItemImplInfo {
    /// The type for which this `impl` is written.
    pub ty: Type,
    /// Info extracted for each public method.
    pub methods: Vec<ImplItemMethodInfo>,
}

impl ItemImplInfo {
    pub fn new(original: &mut ItemImpl) -> syn::Result<Self> {
        if !original.generics.params.is_empty() {
            return Err(Error::new(
                original.generics.params.span(),
                "Impl type parameters are not supported for smart contracts.",
            ));
        }
        let is_trait_impl = original.trait_.is_some();
        let ty = (*original.self_ty.as_ref()).clone();

        let mut methods = vec![];
        for subitem in &mut original.items {
            if let ImplItem::Fn(m) = subitem {
                if let Some(method_info) = ImplItemMethodInfo::new(m, is_trait_impl, ty.clone())? {
                    methods.push(method_info);
                }
            }
        }
        Ok(Self { ty, methods })
    }
}
